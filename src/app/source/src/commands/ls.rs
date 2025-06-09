use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;
use chrono::{DateTime, Local};
use std::fmt::Write as _;

pub struct LsCommand;

const LS_VERSION: &str = "ls 1.0.0";
const LS_HELP: &str = "Usage: ls [OPTION]... [FILE]...\nList information about the FILEs (the current directory by default).\n\n  -a             do not ignore entries starting with .\n  -l             use a long listing format\n  -1             list one file per line\n      --help     display this help and exit\n      --version  output version information and exit";

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn mode_string(node: &VfsNode) -> String {
    fn bits_to_rwx(bits: u8) -> String {
        let r = if bits & 0b100 != 0 { 'r' } else { '-' };
        let w = if bits & 0b010 != 0 { 'w' } else { '-' };
        let x = if bits & 0b001 != 0 { 'x' } else { '-' };
        format!("{}{}{}", r, w, x)
    }
    match node {
        VfsNode::Directory { permissions, .. } => {
            format!("d{}{}{}", bits_to_rwx(permissions.user), bits_to_rwx(permissions.group), bits_to_rwx(permissions.other))
        }
        VfsNode::File { permissions, .. } => {
            format!("-{}{}{}", bits_to_rwx(permissions.user), bits_to_rwx(permissions.group), bits_to_rwx(permissions.other))
        }
        VfsNode::Symlink { permissions, .. } => {
            format!("l{}{}{}", bits_to_rwx(permissions.user), bits_to_rwx(permissions.group), bits_to_rwx(permissions.other))
        }
    }
}

fn node_type_char(node: &VfsNode) -> char {
    match node {
        VfsNode::Directory { .. } => 'd',
        VfsNode::File { .. } => '-',
        VfsNode::Symlink { .. } => 'l',
    }
}

fn format_time(dt: &DateTime<Local>) -> String {
    dt.format("%b %e %H:%M").to_string()
}



fn get_type_char(node: &VfsNode) -> char {
    match node {
        VfsNode::Directory { .. } => 'd',
        VfsNode::File { .. } => '-',
        VfsNode::Symlink { .. } => 'l',
    }
}

impl Command for LsCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle help/version flags - quick exit
        if args.iter().any(|a| a == "--help") {
            return Ok(LS_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(LS_VERSION.to_string());
        }
        
        // parse args - boring flag stuff
        let mut show_all = false;
        let mut long = false;
        let mut one_per_line = false;
        let mut paths = vec![];
        
        for arg in args {
            if arg.starts_with('-') && arg.len() > 1 {
                // handle flags like -a, -l, etc
                for c in arg.chars().skip(1) {
                    match c {
                        'a' => show_all = true,
                        'l' => long = true,
                        '1' => one_per_line = true,
                        _ => {}, // meh, ignore unknown flags
                    }
                }
            } else {
                // not a flag? must be a path
                paths.push(arg.clone());
            }
        }
        
        // default to cwd if no path given
        let path = if paths.is_empty() {
            ctx.cwd.as_str()
        } else {
            paths[0].as_str()
        };
        
        // bail if path doesn't exist
        let node = ctx.vfs.resolve_path(path).ok_or("ls: cannot access: No such file or directory")?;
        
        // collect entries to display
        let mut entries = vec![];
        match node {
            VfsNode::Directory { children, .. } => {
                // for dirs, list all children (maybe hiding dot files)
                for (name, node) in children.iter() {
                    if !show_all && is_hidden(name) {
                        continue;
                    }
                    entries.push((name, node));
                }
            }
            // single file/symlink case - just list the thing itself
            VfsNode::File { name, .. } | VfsNode::Symlink { name, .. } => {
                entries.push((name, node));
            }
        }
        
        // sort by name - users expect alphabetical
        entries.sort_by(|a, b| a.0.cmp(b.0));
        
        // output formatting time - ugh
        let mut out = String::new();
        if long {
            // long format - all the details nobody reads
            for (name, node) in &entries {
                let mode = mode_string(node);
                let nlink = 1; // fake hardlink count
                let owner = "user"; // fake owner
                let group = "group"; // fake group
                let size = match node {
                    VfsNode::File { content, .. } => content.len(),
                    _ => 0, // dirs/symlinks have 0 size
                };
                let mtime = match node {
                    VfsNode::File { mtime, .. } | VfsNode::Directory { mtime, .. } | VfsNode::Symlink { mtime, .. } => format_time(mtime),
                };
                writeln!(out, "{} {:>2} {:<8} {:<8} {:>5} {} {}", mode, nlink, owner, group, size, mtime, name).unwrap();
            }
        } else if one_per_line {
            // one per line - dead simple
            for (name, _) in &entries {
                writeln!(out, "{}", name).unwrap();
            }
        } else {
            // multi-column - not fancy, just hardcoded cols
            let cols = 3;
            for (i, (name, _)) in entries.iter().enumerate() {
                write!(out, "{:<20}", name).unwrap();
                if (i + 1) % cols == 0 {
                    out.push('\n');
                }
            }
            // make sure output ends with newline
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        
        Ok(out)
    }
    }
}

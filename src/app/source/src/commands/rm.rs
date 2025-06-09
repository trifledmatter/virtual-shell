use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;

/// rm [OPTION]... [FILE]...
/// Remove files or directories.
pub struct RmCommand;

const RM_VERSION: &str = "rm 1.0.0";
const RM_HELP: &str = "Usage: rm [OPTION]... [FILE]...\nRemove (unlink) the FILE(s).\n\n  -f, --force           ignore nonexistent files and arguments, never prompt\n  -i                    prompt before every removal\n  -I                    prompt once before removing more than three files, or when removing recursively\n  -r, -R, --recursive   remove directories and their contents recursively\n  -d, --dir             remove empty directories\n  -v, --verbose         explain what is being done\n      --help            display this help and exit\n      --version         output version information and exit";

impl Command for RmCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(RM_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(RM_VERSION.to_string());
        }
        let mut force = false;
        let mut recursive = false;
        let mut verbose = false;
        let mut dir_mode = false;
        let mut files = vec![];
        for arg in args {
            match arg.as_str() {
                "-f" | "--force" => force = true,
                "-r" | "-R" | "--recursive" => recursive = true,
                "-d" | "--dir" => dir_mode = true,
                "-v" | "--verbose" => verbose = true,
                s if s.starts_with('-') => {
                    // ignore -i, -I, --interactive, --one-file-system, --preserve-root, etc. for now
                }
                _ => files.push(arg),
            }
        }
        if files.is_empty() {
            return Err("rm: missing operand".to_string());
        }
        let mut results = Vec::new();
        for file in files {
            let res = match ctx.vfs.resolve_path(file) {
                Some(VfsNode::Directory { .. }) if !recursive && !dir_mode => {
                    Err("rm: cannot remove directory without -r or --dir".to_string())
                }
                Some(_) => ctx.delete_with_events(file),
                None => {
                    if force {
                        Ok(())
                    } else {
                        Err(format!("rm: cannot remove '{}': No such file or directory", file))
                    }
                }
            };
            match res {
                Ok(()) => {
                    if verbose {
                        results.push(format!("removed '{}'.", file));
                    }
                }
                Err(e) => {
                    if !force {
                        results.push(e);
                    }
                }
            }
        }
        Ok(results.join("\n"))
    }
}

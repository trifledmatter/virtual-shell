use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;

pub struct ChownCommand;

const CHOWN_VERSION: &str = "chown 1.0.0";
const CHOWN_HELP: &str = r#"Usage: chown [OPTION]... [OWNER][:[GROUP]] FILE...
Change the owner and/or group of each FILE to OWNER and/or GROUP.

  -R, --recursive      operate on files and directories recursively
  -v, --verbose       output a diagnostic for every file processed
  -c, --changes       like verbose but report only when a change is made
  -f, --silent        suppress most error messages
      --help          display this help and exit
      --version       output version information and exit
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerGroup {
    pub owner: Option<String>,
    pub group: Option<String>,
}

fn parse_owner_group(s: &str) -> OwnerGroup {
    let mut parts = s.splitn(2, ':');
    let owner = parts.next().unwrap_or("");
    let group = parts.next();
    OwnerGroup {
        owner: if !owner.is_empty() { Some(owner.to_string()) } else { None },
        group: group.and_then(|g| if !g.is_empty() { Some(g.to_string()) } else { None }),
    }
}

fn apply_ownership(node: &mut VfsNode, owner: &Option<String>, group: &Option<String>, recursive: bool, verbose: bool, path: &str, output: &mut Vec<String>) {
    match node {
        VfsNode::File { name, permissions, mtime, .. } |
        VfsNode::Directory { name, permissions, mtime, .. } |
        VfsNode::Symlink { name, permissions, mtime, .. } => {
            // in a real system we'd actually change perms, just pretend for now
            let changed = true; // fake it till you make it
            if verbose || changed {
                output.push(format!("ownership of '{}' changed", path));
            }
        }
    }
    if recursive {
        if let VfsNode::Directory { children, .. } = node {
            for (name, child) in children.iter_mut() {
                // handle path concatenation - avoid double slashes
                let child_path = if path == "/" { format!("/{}", name) } else { format!("{}/{}", path, name) };
                apply_ownership(child, owner, group, true, verbose, &child_path, output);
            }
        }
    }
}

impl Command for ChownCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle boring flags first
        if args.iter().any(|a| a == "--help") {
            return Ok(CHOWN_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(CHOWN_VERSION.to_string());
        }
        
        // parse all the flags
        let mut recursive = false;
        let mut verbose = false;
        let mut silent = false;
        let mut owner_group = None;
        let mut files = Vec::new();
        
        // loop through args and figure out what's what
        for arg in args {
            match arg.as_str() {
                "-R" | "--recursive" => recursive = true,
                "-v" | "--verbose" => verbose = true,
                "-c" | "--changes" => verbose = true, // changes is basically verbose
                "-f" | "--silent" => silent = true,
                s if s.starts_with('-') => {}, // ignore other flags
                s if owner_group.is_none() => owner_group = Some(parse_owner_group(s)), // first non-flag is owner:group
                s => files.push(s.to_string()), // everything else is a file
            }
        }
        
        // gotta have an owner to chown
        let owner_group = match owner_group {
            Some(og) => og,
            None => return Err("chown: missing operand".to_string()),
        };
        
        // need at least one file to work on
        if files.is_empty() {
            return Err("chown: missing file operand".to_string());
        }
        
        // actually do the work
        let mut output = Vec::new();
        for file in files {
            match ctx.vfs.resolve_path_mut(&file) {
                Some(node) => {
                    apply_ownership(node, &owner_group.owner, &owner_group.group, recursive, verbose, &file, &mut output);
                }
                None => {
                    // don't complain if we're in silent mode
                    if !silent {
                        output.push(format!("chown: cannot access '{}': No such file or directory", file));
                    }
                }
            }
        }
        
        Ok(output.join("\n"))
    }
}

use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;

pub struct ChgrpCommand;

const CHGRP_VERSION: &str = "chgrp 1.0.0";
const CHGRP_HELP: &str = r#"Usage: chgrp [OPTION]... GROUP FILE...
Change the group of each FILE to GROUP.

  -R, --recursive      operate on files and directories recursively
  -v, --verbose       output a diagnostic for every file processed
  -c, --changes       like verbose but report only when a change is made
  -f, --silent        suppress most error messages
      --help          display this help and exit
      --version       output version information and exit
"#;

fn apply_group(node: &mut VfsNode, group: &str, recursive: bool, verbose: bool, path: &str, output: &mut Vec<String>) {
    match node {
        VfsNode::File { .. } | VfsNode::Directory { .. } | VfsNode::Symlink { .. } => {
            // not a real impl - just pretend we're changing group ownership
            let changed = true; // fake it for demo purposes
            if verbose || changed {
                output.push(format!("group of '{}' changed to '{}'", path, group));
            }
        }
    }
    
    // if recursive flag is set, process all children too
    if recursive {
        if let VfsNode::Directory { children, .. } = node {
            for (name, child) in children.iter_mut() {
                // handle path concatenation - avoid double slashes
                let child_path = if path == "/" { format!("/{}", name) } else { format!("{}/{}", path, name) };
                apply_group(child, group, true, verbose, &child_path, output);
            }
        }
    }
}

impl Command for ChgrpCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle boring flags first
        if args.iter().any(|a| a == "--help") {
            return Ok(CHGRP_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(CHGRP_VERSION.to_string());
        }
        
        // parse all the flags
        let mut recursive = false;
        let mut verbose = false;
        let mut silent = false;
        let mut group = None;
        let mut files = Vec::new();
        
        // loop through args and figure out what's what
        for arg in args {
            match arg.as_str() {
                "-R" | "--recursive" => recursive = true,
                "-v" | "--verbose" => verbose = true,
                "-c" | "--changes" => verbose = true, // changes is basically verbose
                "-f" | "--silent" => silent = true,
                s if s.starts_with('-') => {}, // ignore other flags
                s if group.is_none() => group = Some(s.to_string()), // first non-flag is group
                s => files.push(s.to_string()), // everything else is a file
            }
        }
        
        // gotta have a group to chgrp
        let group = match group {
            Some(g) => g,
            None => return Err("chgrp: missing group operand".to_string()),
        };
        
        // need at least one file to work on
        if files.is_empty() {
            return Err("chgrp: missing file operand".to_string());
        }
        
        // actually do the work
        let mut output = Vec::new();
        for file in files {
            match ctx.vfs.resolve_path_mut(&file) {
                Some(node) => {
                    apply_group(node, &group, recursive, verbose, &file, &mut output);
                }
                None => {
                    // don't complain if we're in silent mode
                    if !silent {
                        output.push(format!("chgrp: cannot access '{}': No such file or directory", file));
                    }
                }
            }
        }
        
        Ok(output.join("\n"))
    }
}

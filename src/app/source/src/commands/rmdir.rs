use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;

/// rmdir [OPTION]... DIRECTORY...
/// Remove the DIRECTORY(ies), if they are empty.
pub struct RmdirCommand;

// basic version and help info - copy/pasted from real rmdir
const RMDIR_VERSION: &str = "rmdir 1.0.0";
const RMDIR_HELP: &str = "Usage: rmdir [OPTION]... DIRECTORY...\nRemove the DIRECTORY(ies), if they are empty.\n\n      --ignore-fail-on-non-empty  ignore each failure to remove a non-empty directory\n  -p, --parents                   remove DIRECTORY and its ancestors\n  -v, --verbose                   output a diagnostic for every directory processed\n      --help                      display this help and exit\n      --version                   output version information and exit";

impl Command for RmdirCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle help/version flags first - quick exit
        if args.iter().any(|a| a == "--help") {
            return Ok(RMDIR_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(RMDIR_VERSION.to_string());
        }
        
        // parse flags
        let mut ignore_fail_on_non_empty = false;
        let mut parents = false;
        let mut verbose = false;
        let mut dirs = vec![];
        
        // process args
        for arg in args {
            match arg.as_str() {
                "--ignore-fail-on-non-empty" => ignore_fail_on_non_empty = true,
                "-p" | "--parents" => parents = true,
                "-v" | "--verbose" => verbose = true,
                s if s.starts_with('-') => {
                    return Err(format!("rmdir: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => dirs.push(arg), // anything else is a dir to remove
            }
        }
        
        // need at least one dir to remove
        if dirs.is_empty() {
            return Err("rmdir: missing operand".to_string());
        }
        
        // collect results to output at end
        let mut results = Vec::new();
        
        // try to remove each requested dir
        for dir in dirs {
            let mut removed = Vec::new();
            let mut current = dir.as_str();
            
            loop {
                match try_remove_dir(ctx, current) {
                    Ok(()) => {
                        // success - log if verbose
                        if verbose {
                            results.push(format!("rmdir: removed directory '{}'.", current));
                        }
                        removed.push(current.to_string());
                        
                        // if not removing parents, we're done
                        if !parents { break; }
                        
                        // try to remove parent dir next
                        if let Some(parent) = parent_path(current) {
                            current = parent;
                        } else {
                            // no more parents to remove
                            break;
                        }
                    }
                    Err(e) => {
                        // special case - ignore non-empty dirs if flag set
                        if ignore_fail_on_non_empty && e == "Directory not empty" {
                            break;
                        } else {
                            // log error and stop
                            results.push(format!("rmdir: failed to remove '{}': {}", current, e));
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(results.join("\n"))
    }
}

// helper to remove a single dir - only if it's empty
fn try_remove_dir(ctx: &mut TerminalContext, path: &str) -> Result<(), String> {
    // Check if target exists and is an empty dir first
    match ctx.vfs.resolve_path(path) {
        Some(VfsNode::Directory { children, .. }) if children.is_empty() => {
            // found empty dir - delete it using the context's delete_with_events method
            ctx.delete_with_events(path)
        }
        Some(VfsNode::Directory { .. }) => Err("Directory not empty".to_string()),
        Some(_) => Err("Not a directory".to_string()),
        None => Err("No such directory".to_string()),
    }
}

// get parent path or none if at root
fn parent_path(path: &str) -> Option<&str> {
    let path = path.trim_matches('/');
    match path.rfind('/') {
        Some(0) => Some("/"), // root dir
        Some(idx) => Some(&path[..idx]), // parent path
        None => None, // no parent (already at root)
    }
}

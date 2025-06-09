use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

const MK_VERSION: &str = "mk 1.0.0";
const MK_HELP: &str = "Usage: mk <file|dir> <path>\nDirectly creates a file (empty) or directory at the given path, no checks, no content, no parent creation, no overwrite protection.\n\n  --help        display this help and exit\n  --version     output version information and exit";

/// mk <file|dir> <path>
/// Directly creates a file (empty) or directory at the given path, no checks, no content, no parent creation, no overwrite protection.
pub struct MkCommand;

impl Command for MkCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // quick exit for help/version flags
        if args.iter().any(|a| a == "--help") {
            return Ok(MK_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(MK_VERSION.to_string());
        }
        
        // need exactly 2 args: type and path
        if args.len() != 2 {
            return Err("Usage: mk <file|dir> <path>".to_string());
        }
        
        let kind = args[0].as_str();
        let path = args[1].as_str();
        
        match kind {
            "file" => {
                // brute force file creation - just shove it in root dir
                // no parent dirs, no checks, just raw creation
                if let Some(parent) = ctx.vfs.resolve_path_mut("/") {
                    if let crate::vfs::VfsNode::Directory { children, .. } = parent {
                        children.insert(path.to_string(), crate::vfs::VfsNode::File {
                            name: path.to_string(),
                            content: Vec::new(),  // empty file
                            permissions: crate::vfs::Permissions::default_file(),
                            mtime: chrono::Local::now(),
                        });
                        return Ok(format!("raw file created: {}", path));
                    }
                }
                Err("mk: could not create file".to_string())
            }
            "dir" => {
                // same deal but for dirs - just jam it in the root
                if let Some(parent) = ctx.vfs.resolve_path_mut("/") {
                    if let crate::vfs::VfsNode::Directory { children, .. } = parent {
                        children.insert(path.to_string(), crate::vfs::VfsNode::Directory {
                            name: path.to_string(),
                            children: std::collections::HashMap::new(),  // empty dir
                            permissions: crate::vfs::Permissions::default_dir(),
                            mtime: chrono::Local::now(),
                        });
                        return Ok(format!("raw dir created: {}", path));
                    }
                }
                Err("mk: could not create dir".to_string())
            }
            _ => Err("mk: first argument must be 'file' or 'dir'".to_string()),
        }
    }
}

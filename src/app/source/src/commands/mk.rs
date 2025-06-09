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
                // brute force file creation - create empty file in root dir
                let full_path = format!("/{}", path.trim_start_matches('/'));
                match ctx.create_file_with_events(&full_path, &[]) {
                    Ok(_) => Ok(format!("raw file created: {}", full_path)),
                    Err(e) => Err(format!("mk: could not create file: {}", e)),
                }
            }
            "dir" => {
                // brute force directory creation - create empty dir in root
                let full_path = format!("/{}", path.trim_start_matches('/'));
                match ctx.create_dir_with_events(&full_path) {
                    Ok(_) => Ok(format!("raw dir created: {}", full_path)),
                    Err(e) => Err(format!("mk: could not create dir: {}", e)),
                }
            }
            _ => Err("mk: first argument must be 'file' or 'dir'".to_string()),
        }
    }
}

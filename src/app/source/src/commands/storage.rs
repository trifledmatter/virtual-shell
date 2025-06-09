use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct StorageCommand;

const STORAGE_VERSION: &str = "storage 1.0.0";
const STORAGE_HELP: &str = r#"Usage: storage COMMAND [OPTIONS]
Manage persistent file system storage with compression.

Note: Auto-save and auto-load are enabled by default.
All file changes are automatically saved to IndexedDB.

Commands:
  save           Manually save current VFS (usually automatic)
  load           Manually reload VFS from storage (destructive!)
  stats          Show storage statistics and compression info
  clear          Clear all persistent storage (reset filesystem)
  autosave       Show auto-save status (always enabled)

Options:
      --help     display this help and exit
      --version  output version information and exit

Examples:
  storage stats           # Show storage usage and compression ratios
  storage save            # Force manual save (redundant)
  storage load            # Reload from storage (overwrites current state!)
  storage clear --force   # Reset to empty filesystem
"#;

impl Command for StorageCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle help and version flags
        if args.iter().any(|a| a == "--help") {
            return Ok(STORAGE_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(STORAGE_VERSION.to_string());
        }

        if args.is_empty() {
            return Err("storage: missing command argument\nTry 'storage --help' for more information.".to_string());
        }

        match args[0].as_str() {
            "save" => {
                // signal that manual storage save is needed
                ctx.set_var("_storage_action", "manual_save");
                Ok("__STORAGE_MANUAL_SAVE__".to_string()) // special marker for frontend
            }
            "load" => {
                // signal that manual storage load is needed
                ctx.set_var("_storage_action", "manual_reload");
                Ok("__STORAGE_MANUAL_RELOAD__".to_string()) // special marker for frontend
            }
            "stats" => {
                // signal that storage stats are needed
                ctx.set_var("_storage_action", "stats");
                Ok("__STORAGE_STATS__".to_string()) // special marker for frontend
            }
            "clear" => {
                // confirm before clearing
                if args.len() > 1 && args[1] == "--force" {
                    ctx.set_var("_storage_action", "clear");
                    Ok("__STORAGE_CLEAR__".to_string()) // special marker for frontend
                } else {
                    Ok("this will permanently delete all stored files!\nuse 'storage clear --force' to confirm.".to_string())
                }
            }
            "autosave" => {
                // auto-save is always enabled now
                Ok("auto-save is permanently enabled. all file changes are automatically saved to indexeddb.".to_string())
            }
            _ => {
                Err(format!("storage: unknown command '{}'\ntry 'storage --help' for more information.", args[0]))
            }
        }
    }
} 
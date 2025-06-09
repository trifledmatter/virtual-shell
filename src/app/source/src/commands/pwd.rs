use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

/// pwd [OPTION]...
/// Print the full filename of the current working directory.
pub struct PwdCommand;

const PWD_VERSION: &str = "pwd 1.0.0";
const PWD_HELP: &str = "Usage: pwd [OPTION]...\nPrint the full filename of the current working directory.\n\n  -L, --logical   use PWD from environment, even if it contains symlinks\n  -P, --physical  resolve all symlinks\n      --help      display this help and exit\n      --version   output version information and exit";

impl Command for PwdCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(PWD_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(PWD_VERSION.to_string());
        }
        let mut logical = true;
        for arg in args {
            match arg.as_str() {
                "-L" | "--logical" => logical = true,
                "-P" | "--physical" => logical = false,
                _ => {},
            }
        }
        // If physical, resolve symlinks in cwd
        if logical {
            Ok(ctx.cwd.clone() + "\n")
        } else {
            // use VFS to resolve symlinks in cwd
            let resolved = ctx.vfs.resolve_path_with_symlinks(&ctx.cwd, true)
                .map(|_| ctx.cwd.clone())
                .unwrap_or_else(|| "/".to_string());
            Ok(resolved + "\n")
        }
    }
}

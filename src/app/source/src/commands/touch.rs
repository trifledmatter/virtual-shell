use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::{VfsNode, Permissions};
use chrono::Local;

/// touch [OPTION]... FILE...
/// Create the FILE(s) if they do not exist, or update the modification time if they do.
pub struct TouchCommand;

const TOUCH_VERSION: &str = "touch 1.0.0";
const TOUCH_HELP: &str = "Usage: touch [OPTION]... FILE...\nUpdate the access and modification times of each FILE to the current time.\n\n  -a         change only the access time\n  -m         change only the modification time\n      --help     display this help and exit\n      --version  output version information and exit";

impl Command for TouchCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(TOUCH_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(TOUCH_VERSION.to_string());
        }
        let mut files = vec![];
        let mut only_mtime = false;
        let mut only_atime = false;
        for arg in args {
            match arg.as_str() {
                "-a" => only_atime = true,
                "-m" => only_mtime = true,
                s if s.starts_with('-') => {
                    return Err(format!("touch: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => files.push(arg),
            }
        }
        if files.is_empty() {
            return Err("touch: missing file operand".to_string());
        }
        let mut results = Vec::new();
        for file in files {
            let now = Local::now();
            match ctx.vfs.resolve_path_mut(file) {
                Some(VfsNode::File { mtime, .. }) => {
                    if !only_atime { *mtime = now; }
                    // (no atime in this VFS)
                }
                Some(VfsNode::Directory { .. }) => {
                    results.push(format!("touch: '{}' is a directory", file));
                }
                Some(VfsNode::Symlink { mtime, .. }) => {
                    if !only_atime { *mtime = now; }
                }
                None => {
                    // Create the file with events
                    ctx.create_file_with_events(file, &[])?;
                }
            }
        }
        if results.is_empty() {
            Ok(String::new())
        } else {
            Ok(results.join("\n"))
        }
    }
}

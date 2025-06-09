use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

/// ln -s TARGET LINK_NAME
/// Make a symbolic link to TARGET named LINK_NAME.
pub struct LnCommand;

const LN_VERSION: &str = "ln 1.0.0";
const LN_HELP: &str = "Usage: ln -s TARGET LINK_NAME\nMake a symbolic link to TARGET named LINK_NAME.\n\n  -s             make symbolic links instead of hard links\n      --help     display this help and exit\n      --version  output version information and exit";

impl Command for LnCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(LN_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(LN_VERSION.to_string());
        }
        let mut symbolic = false;
        let mut rest = vec![];
        for arg in args {
            if arg == "-s" {
                symbolic = true;
            } else {
                rest.push(arg);
            }
        }
        if !symbolic {
            return Err("ln: only symbolic links (-s) are supported in this VFS".to_string());
        }
        if rest.len() != 2 {
            return Err("Usage: ln -s TARGET LINK_NAME".to_string());
        }
        let target = &rest[0];
        let link_name = &rest[1];
        ctx.vfs.create_symlink(link_name, target)?;
        Ok(String::new())
    }
}

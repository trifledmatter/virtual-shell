use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct UnaliasCommand;

impl Command for UnaliasCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            return Err("unalias: usage: unalias [-a] name [name ...]".to_string());
        }
        let mut exit_code = 0;
        if args[0] == "-a" {
            ctx.aliases.clear();
            return Ok(String::new());
        }
        for name in args {
            if ctx.aliases.remove(name).is_none() {
                exit_code = 1;
            }
        }
        if exit_code == 0 {
            Ok(String::new())
        } else {
            Err("unalias: one or more names not found".to_string())
        }
    }
}

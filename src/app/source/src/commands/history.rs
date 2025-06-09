use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct HistoryCommand;

impl Command for HistoryCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            let out = ctx.history.iter().enumerate().map(|(i, cmd)| format!("{:4}  {}", i+1, cmd)).collect::<Vec<_>>().join("\n");
            Ok(out)
        } else if args.len() == 1 && args[0] == "-c" {
            ctx.history.clear();
            Ok("History cleared".to_string())
        } else {
            Err("history: usage: history [-c]".to_string())
        }
    }
}

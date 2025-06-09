use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct ClearCommand;

impl Command for ClearCommand {
    fn execute(&self, _args: &[String], _ctx: &mut TerminalContext) -> CommandResult {
        // output a special marker string for the frontend to detect and clear the screen
        Ok("__CLEAR_SCREEN__".to_string())
    }
}
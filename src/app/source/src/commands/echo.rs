use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

/// echo [STRING]...
/// Write arguments to the standard output.
pub struct EchoCommand;

const ECHO_VERSION: &str = "echo 1.0.0";
const ECHO_HELP: &str = "Usage: echo [STRING]...\nWrite arguments to the standard output, separated by spaces and followed by a newline.\n\n  -n             do not output the trailing newline\n      --help     display this help and exit\n      --version  output version information and exit";

impl Command for EchoCommand {
    fn execute(&self, args: &[String], _ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(ECHO_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(ECHO_VERSION.to_string());
        }
        let mut n_flag = false;
        let mut output = Vec::new();
        for arg in args {
            if arg == "-n" {
                n_flag = true;
            } else {
                output.push(arg.as_str());
            }
        }
        let mut out = output.join(" ");
        if !n_flag {
            out.push('\n');
        }
        Ok(out)
    }
}

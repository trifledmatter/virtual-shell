use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::commands::ps::VirtualProcess;

pub struct KillallCommand;

const KILLALL_VERSION: &str = "killall 1.0.0";
const KILLALL_HELP: &str = r#"Usage: killall [options] <name> [...]
Send a signal to all processes running any of the specified commands.

  -s, --signal SIGNAL   specify the signal to send (default: TERM)
  -l, --list            list signal names
      --help            display this help and exit
      --version         output version information and exit

This is a virtual shell. Only simulated processes are affected.
"#;

const SIGNALS: &[&str] = &["HUP", "INT", "QUIT", "ILL", "ABRT", "FPE", "KILL", "SEGV", "PIPE", "ALRM", "TERM", "USR1", "USR2", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU"];

fn get_virtual_processes(ctx: &TerminalContext) -> Vec<VirtualProcess> {
    // Use the same as ps
    crate::commands::ps::get_virtual_processes(ctx)
}

impl Command for KillallCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(KILLALL_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(KILLALL_VERSION.to_string());
        }
        if args.iter().any(|a| a == "-l" || a == "--list") {
            return Ok(SIGNALS.join(" "));
        }
        let mut signal = "TERM";
        let mut names = Vec::new();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-s" | "--signal" => {
                    i += 1;
                    if i < args.len() {
                        signal = &args[i];
                    } else {
                        return Err("killall: option requires an argument -- 's'".to_string());
                    }
                }
                s if s.starts_with('-') => {},
                s => names.push(s.to_string()),
            }
            i += 1;
        }
        if names.is_empty() {
            return Err("killall: missing process name operand".to_string());
        }
        let procs = get_virtual_processes(ctx);
        let mut output = Vec::new();
        for name in &names {
            let mut found = false;
            for p in &procs {
                if &p.cmd == name {
                    output.push(format!("Sent signal {} to {} (pid {})", signal, name, p.pid));
                    found = true;
                }
            }
            if !found {
                output.push(format!("killall: no process found with name '{}'", name));
            }
        }
        Ok(output.join("\n"))
    }
}

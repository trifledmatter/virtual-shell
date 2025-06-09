use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct KillCommand;

const KILL_VERSION: &str = "kill 1.0.0";
const KILL_HELP: &str = r#"Usage: kill [options] <pid> [...]
Send a signal to a process.

  -s, --signal SIGNAL   specify the signal to send (default: TERM)
  -l, --list            list signal names
      --help            display this help and exit
      --version         output version information and exit

This is a virtual shell. Only simulated processes are affected.
"#;

const SIGNALS: &[&str] = &["HUP", "INT", "QUIT", "ILL", "ABRT", "FPE", "KILL", "SEGV", "PIPE", "ALRM", "TERM", "USR1", "USR2", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU"];

impl Command for KillCommand {
    fn execute(&self, args: &[String], _ctx: &mut TerminalContext) -> CommandResult {
        // quick exits for help, version and signal list
        if args.iter().any(|a| a == "--help") {
            return Ok(KILL_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(KILL_VERSION.to_string());
        }
        if args.iter().any(|a| a == "-l" || a == "--list") {
            return Ok(SIGNALS.join(" ")); // just dump all signals
        }
        
        // defaults
        let mut signal = "TERM"; // default signal
        let mut pids = Vec::new();
        
        // parse args manually cuz why not
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-s" | "--signal" => {
                    // grab next arg as signal value
                    i += 1;
                    if i < args.len() {
                        signal = &args[i];
                    } else {
                        return Err("kill: option requires an argument -- 's'".to_string());
                    }
                }
                s if s.starts_with('-') => {}, // ignore other flags
                s => {
                    // anything else should be a pid
                    if let Ok(pid) = s.parse::<u32>() {
                        pids.push(pid);
                    } else {
                        return Err(format!("kill: invalid pid '{}': not a number", s));
                    }
                }
            }
            i += 1; // next arg
        }
        
        // gotta have something to kill
        if pids.empty() {
            return Err("kill: missing pid operand".to_string());
        }
        
        // fake it till you make it
        // TODO: impl. real
        let mut output = Vec::new();
        for pid in pids {
            output.push(format!("Sent signal {} to pid {}", signal, pid));
        }
        Ok(output.join("\n")) // one msg per line
    }
}

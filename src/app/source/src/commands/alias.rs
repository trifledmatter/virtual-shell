use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct AliasCommand;

fn shell_quote(s: &str) -> String {
    // wrap string in single quotes, handle escaping
    // typical posix shell quoting - works for bash/zsh/etc
    let mut quoted = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            quoted.push_str("'\\''"); // escape single quote with '\'', hack but works
        } else {
            quoted.push(c);
        }
    }
    quoted.push('\'');
    quoted
}

impl Command for AliasCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            // no args = show all aliases
            let mut out = Vec::new();
            for (k, v) in ctx.aliases.iter() {
                out.push(format!("{}={}", k, shell_quote(v)));
            }
            out.sort(); // alphabetical, why not
            return Ok(out.join("\n"));
        }
        
        // with args = set/get specific aliases
        let mut output = Vec::new();
        let mut exit_code = 0;
        
        for arg in args {
            if let Some(eq) = arg.find('=') {
                // got an equals sign = setting an alias
                let name = &arg[..eq];
                let value = &arg[eq+1..];
                ctx.aliases.insert(name.to_string(), value.to_string());
            } else {
                // no equals = lookup existing alias
                if let Some(val) = ctx.aliases.get(arg) {
                    output.push(format!("{}={}", arg, shell_quote(val)));
                } else {
                    // not found = error
                    output.push(format!("alias: {}: not found", arg));
                    exit_code = 1; // unix-y error code
                }
            }
        }
        
        // return ok or err based on exit code
        if exit_code == 0 {
            Ok(output.join("\n"))
        } else {
            Err(output.join("\n"))
        }
    }
}

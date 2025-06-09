use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct SetCommand;

impl Command for SetCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // no args? just dump all vars and options
        if args.is_empty() {
            let mut out = Vec::new();
            // add all vars first
            for (k, v) in ctx.vars.iter() {
                out.push(format!("{}='{}'", k, v));
            }
            // tack on shell options at the end
            out.push(format!("set -e: {}", ctx.options.errexit));
            out.push(format!("set -x: {}", ctx.options.xtrace));
            return Ok(out.join("\n"));
        }

        // process each arg one by one
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-e" => ctx.options.errexit = true,  // enable errexit
                "+e" => ctx.options.errexit = false, // disable errexit
                "-x" => ctx.options.xtrace = true,   // enable debug trace
                "+x" => ctx.options.xtrace = false,  // disable debug trace
                s if s.contains('=') => {
                    // handle var assignment (foo=bar)
                    let mut parts = s.splitn(2, '=');
                    let name = parts.next().unwrap();
                    let value = parts.next().unwrap_or(""); // empty val is fine
                    ctx.vars.insert(name.to_string(), value.to_string());
                }
                _ => {}, // meh, ignore anything else
            }
            i += 1;
        }
        Ok(String::new()) // nothing to say
    }
}

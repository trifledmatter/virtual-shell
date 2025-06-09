use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct FunctionsCommand;

impl Command for FunctionsCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            // just list all funcs
            let mut out = Vec::new();
            for (name, body) in ctx.functions.iter() {
                out.push(format!("{}() {{ {} }}", name, body));
            }
            return Ok(out.join("\n"));
        }
        
        // got args? try to define a new func
        if args.len() >= 2 {
            let name = &args[0];
            // concat everything after name as body
            let body = args[1..].join(" ");
            // save it and be done
            ctx.functions.insert(name.clone(), body);
            Ok(format!("Function '{}' defined", name))
        } else {
            // not enough args, show usage
            Err("functions: usage: functions [name body]".to_string())
        }
    }
}

use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct ExportCommand;

const EXPORT_HELP: &str = r#"Usage: export name[=word]...
       export -p
Set export attribute for variables (add to environment for child commands).

  -p      print all exported variables
      --help     display this help and exit
"#;

impl Command for ExportCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // show help if asked for
        if args.iter().any(|a| a == "--help") {
            return Ok(EXPORT_HELP.to_string());
        }
        
        // print all vars if -p flag
        if args.iter().any(|a| a == "-p") {
            let mut out = String::new();
            for (k, v) in ctx.env.iter() {
                out.push_str(&format!("export {}={}\n", k, v));
            }
            return Ok(out);
        }
        
        // no args? no problem
        if args.is_empty() {
            // posix says whatever, so we do nothing
            return Ok(String::new());
        }
        
        // track if anything fails
        let mut status = 0;
        
        // process each arg
        for arg in args {
            if let Some(eq) = arg.find('=') {
                // handle var=value format
                let (name, value) = arg.split_at(eq);
                let value = &value[1..]; // skip the '='
                
                // empty name? that's bad
                if name.is_empty() {
                    status = 1;
                    continue;
                }
                
                // set the var
                ctx.env.insert(name.to_string(), value.to_string());
            } else {
                // empty arg? no good
                if arg.is_empty() {
                    status = 1;
                    continue;
                }
                
                // just mark existing var as exported or create empty one
                ctx.env.entry(arg.to_string()).or_insert_with(String::new);
            }
        }
        
        // return empty if all good, error if not
        if status == 0 {
            Ok(String::new())
        } else {
            Err("export: one or more names could not be exported".to_string())
        }
    }
}

use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct EnvCommand;

// version string for --version flag
const ENV_VERSION: &str = "env 1.0.0";
// help text, shown with --help
const ENV_HELP: &str = r#"Usage: env [OPTION]... [-] [NAME=VALUE]... [COMMAND [ARG]...]
Set each NAME to VALUE in the environment and run COMMAND.

  -i, --ignore-environment  start with an empty environment
  -u, --unset=NAME          remove variable from the environment
      --help                display this help and exit
      --version             output version information and exit

If no COMMAND, print the resulting environment.
"#;

impl Command for EnvCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // quick returns for help/version flags
        if args.iter().any(|a| a == "--help") {
            return Ok(ENV_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(ENV_VERSION.to_string());
        }
        
        // work with a copy of the env so we don't mess with the original
        let mut env = ctx.env.clone();
        let mut ignore_env = false;
        let mut unset_vars = Vec::new();
        let mut i = 0;
        
        // process flags and options
        while i < args.len() {
            match args[i].as_str() {
                "-i" | "--ignore-environment" | "-" => ignore_env = true,
                s if s.starts_with("-u") => {
                    // handle -u/--unset with its argument
                    let name = if s == "-u" || s == "--unset" {
                        i += 1;
                        if i < args.len() { &args[i] } else { return Err("env: option requires an argument -- 'u'".to_string()); }
                    } else if let Some(eq) = s.find('=') {
                        &s[eq+1..]
                    } else {
                        &s[2..]
                    };
                    unset_vars.push(name.to_string());
                }
                s if s.starts_with('-') => {}, // ignore unknown flags, whatever
                _ => break, // not a flag, break out
            }
            i += 1;
        }
        
        // apply env modifications
        if ignore_env {
            env.clear(); // nuke it all if -i was given
        }
        
        // remove any vars that were asked to be unset
        for name in unset_vars {
            env.remove(&name);
        }
        
        // collect name=value pairs
        while i < args.len() {
            if let Some(eq) = args[i].find('=') {
                let (name, value) = args[i].split_at(eq);
                let value = &value[1..]; // skip the = sign
                env.insert(name.to_string(), value.to_string());
                i += 1;
            } else {
                break; // not a var assignment, must be command
            }
        }
        
        // no command? just dump the env vars
        if i >= args.len() {
            let mut out = String::new();
            for (k, v) in env.iter() {
                out.push_str(&format!("{}={}\n", k, v));
            }
            return Ok(out);
        }
        
        // if we get here, there's a command to run
        // but we're just simulating it for now
        let cmd = &args[i];
        let cmd_args = &args[i+1..];
        
        // build output showing what would run
        let mut out = format!("Would run: {}", cmd);
        if !cmd_args.is_empty() {
            out.push(' ');
            out.push_str(&cmd_args.join(" "));
        }
        out.push_str("\nWith env:\n");
        for (k, v) in env.iter() {
            out.push_str(&format!("{}={}\n", k, v));
        }
        Ok(out)
    }
}

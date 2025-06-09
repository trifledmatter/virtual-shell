use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use regex::Regex;

pub struct GrepCommand;

impl Command for GrepCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            return Err("Usage: grep PATTERN [FILE]...".to_string());
        }
        
        let pattern = &args[0];
        let regex = match Regex::new(pattern) {
            Ok(r) => r,
            Err(e) => return Err(format!("Invalid regex pattern: {}", e)),
        };
        
        let mut output = Vec::new();
        
        if args.len() == 1 {
            // stdin not implemented, meh
            return Err("Reading from stdin not supported".to_string());
        }
        
        for filename in &args[1..] {
            // handle absolute vs relative paths
            let path = if filename.starts_with('/') {
                filename.to_string()
            } else {
                format!("{}/{}", ctx.cwd, filename)
            };
            
            match ctx.vfs.read_file(&path) {
                Ok(content_bytes) => {
                    // try to parse as utf8, skip if binary garbage
                    if let Ok(content) = String::from_utf8(content_bytes.to_vec()) {
                        for (i, line) in content.lines().enumerate() {
                            // check if line matches our regex
                            if regex.is_match(line) {
                                // include filename if multiple files given
                                if args.len() > 2 {
                                    output.push(format!("{}:{}: {}", filename, i + 1, line));
                                } else {
                                    output.push(format!("{}: {}", i + 1, line));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    // can't read? just show error and move on
                    output.push(format!("grep: {}: {}", filename, e));
                }
            }
        }
        
        // join all matches with newlines
        Ok(output.join("\n"))
    }
}

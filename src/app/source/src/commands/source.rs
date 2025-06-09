use crate::command::{Command, CommandResult, run_command};
use crate::context::TerminalContext;

pub struct SourceCommand;

impl Command for SourceCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // bail if no args given
        if args.is_empty() {
            return Err("source: filename argument required".to_string());
        }
        let filename = &args[0];
        
        // figure out the actual path to the file
        let file_path = if filename.starts_with('/') {
            // absolute path, use as is
            filename.to_string()
        } else if !filename.contains('/') && ctx.env.get("PATH").is_some() {
            // no slashes = look in $PATH first
            let path_env = ctx.env.get("PATH").unwrap();
            let mut found_path = None;
            for dir in path_env.split(':') {
                let full_path = format!("{}/{}", dir, filename);
                if ctx.vfs.resolve_path(&full_path).is_some() {
                    // found it, stop looking
                    found_path = Some(full_path);
                    break;
                }
            }
            // fallback to cwd if not in path
            found_path.unwrap_or(format!("{}/{}", ctx.cwd, filename))
        } else {
            // relative path, prepend cwd
            format!("{}/{}", ctx.cwd, filename)
        };
        
        // try to read the file
        let file_content = match ctx.vfs.read_file(&file_path) {
            Ok(content_bytes) => {
                match String::from_utf8(content_bytes.to_vec()) {
                    Ok(s) => s,
                    Err(_) => return Err("source: file contains invalid UTF-8".to_string()),
                }
            }
            Err(_) => return Err(format!("source: {}: file not found or unreadable", filename)),
        };
        
        // track last cmd result to return at end
        let mut last_result = Ok(String::new());
        
        // borrow checker hack - take ownership of registry temporarily
        let registry = ctx.registry.take()
            .ok_or("source: command registry not available".to_string())?;
        
        // run each line in the script
        for line in file_content.lines() {
            let line = line.trim();
            // skip empty lines and comments
            if line.is_empty() || line.starts_with('#') { continue; }
            last_result = run_command(line, ctx, &registry);
        }
        
        // put the registry back when done
        ctx.registry = Some(registry);
        
        // return result of last command
        last_result
    }
}

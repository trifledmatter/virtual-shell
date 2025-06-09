use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct RawCreateCommand;


const RC_VERSION: &str = "rawcreate 1.0.0";
const RC_HELP: &str = "Usage: rawcreate <path> <hex bytes...>\nCreate a file with raw bytes specified in hex format.\n\n  -h, --help     display this help and exit\n      --version  output version information and exit";

impl Command for RawCreateCommand {
// this command creates a file with raw bytes specified in hex format
// it is very low-level and does not check for anything, which is dangerous

  fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // usage: rawcreate <path> <hex bytes...>
        if args.len() < 2 {
            return Err("rawcreate: need a path and at least one byte".to_string());
        }
        let path = &args[0];
        // parse hex bytes, ignore anything that's not a valid byte
        let mut bytes = Vec::new();
        for s in &args[1..] {
            if let Ok(b) = u8::from_str_radix(s, 16) {
                bytes.push(b);
            } else {
                // whatever, just skip it
            }
        }
        if bytes.is_empty() {
            return Err("rawcreate: no valid bytes given".to_string());
        }
        match ctx.create_file_with_events(path, &bytes) {
            Ok(_) => Ok(format!("made file {} ({} bytes)", path, args.len() - 1)),
            Err(e) => Err(format!("rawcreate: {}", e)),
        }
    }
}

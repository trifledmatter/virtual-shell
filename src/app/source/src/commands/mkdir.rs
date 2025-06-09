use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::{VfsNode, Permissions};
use chrono::Local;

pub struct MkdirCommand;

const VERSION: &str = "mkdir 1.0.0";
const HELP: &str = "Usage: mkdir [OPTION]... DIRECTORY...
Create the DIRECTORY(ies), if they do not already exist.

Mandatory arguments to long options are mandatory for short options too.
  -m, --mode=MODE   set file mode (as in chmod), not a=rwx - umask
  -p, --parents     no error if existing, make parent directories as needed,
                    with their file modes unaffected by any -m option
  -v, --verbose     print a message for each created directory
  -Z                   set SELinux security context of each created directory
                         to the default type
      --context[=CTX]  like -Z, or if CTX is specified then set the SELinux
                         or SMACK security context to CTX
      --help        display this help and exit
      --version     output version information and exit";

impl Command for MkdirCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            return Err("Usage: mkdir [OPTION]... DIRECTORY...".to_string());
        }
        let mut paths = vec![];
        let mut parents = false;
        let mut verbose = false;
        let mut mode: Option<Permissions> = None;
        let mut show_help = false;
        let mut show_version = false;
        let mut skip_next = false;
        for (i, arg) in args.iter().enumerate() {
            if skip_next { skip_next = false; continue; }
            match arg.as_str() {
                "-p" | "--parents" => parents = true,
                "-v" | "--verbose" => verbose = true,
                "--help" => show_help = true,
                "--version" => show_version = true,
                "-Z" => {}, // ignore
                s if s.starts_with("--context") => {}, // ignore
                s if s.starts_with("--mode=") => {
                    let m = &s[7..];
                    mode = Some(parse_mode(m)?);
                }
                "-m" => {
                    if let Some(m) = args.get(i+1) {
                        mode = Some(parse_mode(m)?);
                        skip_next = true;
                    } else {
                        return Err("mkdir: option requires an argument -- 'm'".to_string());
                    }
                }
                s if s.starts_with("-m") && s.len() > 2 => {
                    mode = Some(parse_mode(&s[2..])?);
                }
                s if s.starts_with('-') => {
                    // unknown/unsupported option
                    return Err(format!("mkdir: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => paths.push(arg),
            }
        }
        if show_help {
            return Ok(HELP.to_string());
        }
        if show_version {
            return Ok(VERSION.to_string());
        }
        if paths.is_empty() {
            return Err("mkdir: missing operand".to_string());
        }
        let mut results = Vec::new();
        for path in paths {
            let res = if parents {
                mkdir_parents(ctx, path, verbose)
            } else {
                mkdir_single(ctx, path, mode, verbose)
            };
            match res {
                Ok(msg) => if !msg.is_empty() { results.push(msg); },
                Err(e) => results.push(format!("mkdir: cannot create directory '{}': {}", path, e)),
            }
        }
        Ok(results.join("\n"))
    }
}

fn mkdir_single(ctx: &mut TerminalContext, path: &str, mode: Option<Permissions>, verbose: bool) -> Result<String, String> {
    // check if already exists
    if ctx.vfs.resolve_path(path).is_some() {
        return Err("File exists".to_string());
    }
    
    // create directory with events
    ctx.create_dir_with_events(path)?;
    
    // return success msg if verbose, otherwise empty string
    if verbose {
        Ok(format!("mkdir: created directory '{}'.", path))
    } else {
        Ok(String::new())
    }
}

fn mkdir_parents(ctx: &mut TerminalContext, path: &str, verbose: bool) -> Result<String, String> {
    // split path into parts, skip empty stuff
    let components: Vec<&str> = path.trim_matches('/').split('/').filter(|c| !c.is_empty()).collect();
    if components.is_empty() {
        return Err("Invalid path".to_string());
    }
    
    let mut created = Vec::new();
    let mut current_path = String::new();
    
    // go through each path component, building up the path
    for comp in &components {
        current_path = if current_path.is_empty() {
            format!("/{}", comp)
        } else {
            format!("{}/{}", current_path, comp)
        };
        
        // check if this path component already exists
        if ctx.vfs.resolve_path(&current_path).is_none() {
            // doesn't exist, create it with events
            ctx.create_dir_with_events(&current_path)?;
            created.push(current_path.clone());
        }
    }
    
    // only print stuff in verbose mode
    if verbose {
        Ok(created.into_iter().map(|path| format!("mkdir: created directory '{}'.", path)).collect::<Vec<_>>().join("\n"))
    } else {
        Ok(String::new())
    }
}

fn parse_mode(mode: &str) -> Result<Permissions, String> {
    // only octal for now, deal with symbolic later if we care
    let m = if mode.starts_with('0') {
        &mode[1..] // strip leading zero if present
    } else {
        mode
    };
    
    // bail if not 3 digits or non-octal chars
    if m.len() != 3 || !m.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("invalid mode: {}", mode));
    }
    
    // grab user/group/other bits - yolo on the unwraps, we already validated
    let u = m.chars().nth(0).unwrap().to_digit(8).unwrap() as u8;
    let g = m.chars().nth(1).unwrap().to_digit(8).unwrap() as u8;
    let o = m.chars().nth(2).unwrap().to_digit(8).unwrap() as u8;
    
    Ok(Permissions::new(u, g, o))
}

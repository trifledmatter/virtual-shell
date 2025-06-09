use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::{VfsNode, Permissions};

pub struct ChmodCommand;

const CHMOD_VERSION: &str = "chmod 1.0.0";
const CHMOD_HELP: &str = r#"Usage: chmod [OPTION]... MODE[,MODE]... FILE...
Change the mode of each FILE to MODE.

  -R, --recursive      change files and directories recursively
  -v, --verbose       output a diagnostic for every file processed
  -c, --changes       like verbose but report only when a change is made
  -f, --silent        suppress most error messages
      --help          display this help and exit
      --version       output version information and exit
"#;

fn parse_octal_mode(mode: &str) -> Option<Permissions> {
    let digits = mode.trim_start_matches('0');
    if digits.len() == 3 {
        let u = digits.chars().nth(0)?.to_digit(8)? as u8;
        let g = digits.chars().nth(1)?.to_digit(8)? as u8;
        let o = digits.chars().nth(2)?.to_digit(8)? as u8;
        Some(Permissions::new(u, g, o))
    } else {
        None
    }
}

fn apply_permissions(node: &mut VfsNode, perms: Permissions, recursive: bool, verbose: bool, path: &str, output: &mut Vec<String>) {
    match node {
        VfsNode::File { permissions, .. } | VfsNode::Directory { permissions, .. } => {
            let changed = *permissions != perms;
            *permissions = perms;
            if verbose || changed {
                output.push(format!("mode of '{}' changed", path));
            }
        }
        VfsNode::Symlink { .. } => {}
    }
    if recursive {
        if let VfsNode::Directory { children, .. } = node {
            for (name, child) in children.iter_mut() {
                let child_path = if path == "/" { format!("/{}", name) } else { format!("{}/{}", path, name) };
                apply_permissions(child, perms, true, verbose, &child_path, output);
            }
        }
    }
}

impl Command for ChmodCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(CHMOD_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(CHMOD_VERSION.to_string());
        }
        let mut recursive = false;
        let mut verbose = false;
        let mut silent = false;
        let mut mode = None;
        let mut files = Vec::new();
        for arg in args {
            match arg.as_str() {
                "-R" | "--recursive" => recursive = true,
                "-v" | "--verbose" => verbose = true,
                "-c" | "--changes" => verbose = true,
                "-f" | "--silent" => silent = true,
                s if s.starts_with('-') => {},
                s if mode.is_none() => mode = Some(s.to_string()),
                s => files.push(s.to_string()),
            }
        }
        let mode = match mode {
            Some(m) => m,
            None => return Err("chmod: missing operand".to_string()),
        };
        let perms = match parse_octal_mode(&mode) {
            Some(p) => p,
            None => return Err("chmod: only octal modes supported in this version".to_string()),
        };
        if files.is_empty() {
            return Err("chmod: missing file operand".to_string());
        }
        let mut output = Vec::new();
        for file in files {
            match ctx.vfs.resolve_path_mut(&file) {
                Some(node) => {
                    apply_permissions(node, perms, recursive, verbose, &file, &mut output);
                }
                None => {
                    if !silent {
                        output.push(format!("chmod: cannot access '{}': No such file or directory", file));
                    }
                }
            }
        }
        Ok(output.join("\n"))
    }
}

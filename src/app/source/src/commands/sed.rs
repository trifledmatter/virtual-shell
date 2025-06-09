use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use regex::Regex;

pub struct SedCommand;

const SED_VERSION: &str = "sed 1.0.0";
const SED_HELP: &str = r#"Usage: sed [OPTION]... {script-only-if-no-other-script} [input-file]...
Stream editor for filtering and transforming text.

  -e script, --expression=script  add the script to the commands to be executed
  -n, --quiet, --silent          suppress automatic printing of pattern space
  -E, -r, --regexp-extended      use extended regular expressions in the script
      --help     display this help and exit
      --version  output version information and exit
"#;

impl Command for SedCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(SED_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(SED_VERSION.to_string());
        }

        let mut script = None;
        let mut files = Vec::new();
        let mut suppress_print = false;
        let mut extended = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-n" | "--quiet" | "--silent" => suppress_print = true,
                "-E" | "-r" | "--regexp-extended" => extended = true,
                "-e" | "--expression" => {
                    i += 1;
                    if i < args.len() {
                        script = Some(args[i].clone());
                    } else {
                        return Err("sed: option requires an argument -- 'e'".to_string());
                    }
                }
                "--" => {
                    files.extend_from_slice(&args[i+1..]);
                    break;
                }
                s if s.starts_with('-') => {
                    return Err(format!("sed: unrecognized option '{}'", s));
                }
                s => {
                    if script.is_none() {
                        script = Some(s.to_string());
                    } else {
                        files.push(s.to_string());
                    }
                }
            }
            i += 1;
        }

        let script = match script {
            Some(s) => s,
            None => return Err("sed: no script given".to_string()),
        };
        // only doing s/pattern/replacement/ for now cuz i'm lazy
        let (pat, rep) = if let Some(rest) = script.strip_prefix("s/") {
            // parse into pattern and replacement parts
            let mut parts = rest.splitn(2, '/');
            let pat = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("");
            let mut parts = rest.splitn(2, '/');
            let rep = parts.next().unwrap_or("");
            (pat, rep)
        } else {
            // bail if not s/// format
            return Err("sed: only s/// scripts are supported in this version".to_string());
        };

        // compile regex - extended flag doesn't actually do anything yet lol
        // TODO: make extended mode actually different
        let re = if extended {
            Regex::new(pat)
        } else {
            Regex::new(pat)
        }.map_err(|e| format!("sed: invalid regex: {}", e))?;

        let mut output = String::new();

        // default to stdin if no files given
        let input_files = if files.is_empty() {
            vec!["-".to_string()]
        } else {
            files
        };

        for file in input_files {
            // grab file contents or bail
            let lines: Vec<String> = if file == "-" {
                // stdin not implemented, just return empty for now
                // whatever, we'll fix it later
                vec![]
            } else {
                match ctx.vfs.read_file(&file) {
                    Ok(bytes) => String::from_utf8_lossy(bytes).lines().map(|l| l.to_string()).collect(),
                    Err(e) => return Err(format!("sed: {}: {}", file, e)),
                }
            };

            // do the replacements
            for line in lines {
                let replaced = re.replace_all(&line, rep);
                if !suppress_print {
                    output.push_str(&replaced);
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }
}

use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct PsCommand;

const PS_VERSION: &str = "ps 1.0.0";
const PS_HELP: &str = r#"Usage: ps [option ...]
Report a snapshot of the current processes.

  -e, -A         select all processes
  -f             full-format listing
  -o format      user-defined output format
  -u userlist    select by effective user
  -p pidlist     select by process ID
      --help     display this help and exit
      --version  output version information and exit

This is a virtual shell. Only simulated processes are shown.
"#;

#[derive(Debug, Clone)]
pub struct VirtualProcess {
    pub pid: u32,
    pub ppid: u32,
    pub user: String,
    pub tty: String,
    pub cmd: String,
    pub state: String,
}

pub fn get_virtual_processes(ctx: &TerminalContext) -> Vec<VirtualProcess> {
    // fake processes
    // TODO: keep track of real processing
    vec![
        VirtualProcess { pid: 1, ppid: 0, user: "root".to_string(), tty: "?".to_string(), cmd: "init".to_string(), state: "S".to_string() },
        VirtualProcess { pid: 2, ppid: 1, user: "root".to_string(), tty: "?".to_string(), cmd: "kthreadd".to_string(), state: "S".to_string() },
        VirtualProcess { pid: 100, ppid: 1, user: "user".to_string(), tty: "tty1".to_string(), cmd: "bash".to_string(), state: "S".to_string() },
        VirtualProcess { pid: 101, ppid: 100, user: "user".to_string(), tty: "tty1".to_string(), cmd: "ps".to_string(), state: "R".to_string() },
    ]
}

impl Command for PsCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle help/version flags - quick exit
        if args.iter().any(|a| a == "--help") {
            return Ok(PS_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(PS_VERSION.to_string());
        }
        
        // parse flags
        let mut show_all = false;
        let mut user_filter: Option<String> = None;
        let mut pid_filter: Option<Vec<u32>> = None;
        let mut full = false;
        let mut custom_format: Option<String> = None;
        
        // process args
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-e" | "-A" => show_all = true,
                "-f" => full = true,
                "-o" => {
                    i += 1;
                    if i < args.len() {
                        custom_format = Some(args[i].clone());
                    }
                }
                "-u" => {
                    i += 1;
                    if i < args.len() {
                        user_filter = Some(args[i].clone());
                    }
                }
                "-p" => {
                    i += 1;
                    if i < args.len() {
                        let pids = args[i].split(',').filter_map(|s| s.parse().ok()).collect();
                        pid_filter = Some(pids);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        
        // get and filter processes
        let mut procs = get_virtual_processes(ctx);
        if let Some(user) = user_filter {
            procs.retain(|p| p.user == user);
        }
        if let Some(pids) = pid_filter {
            procs.retain(|p| pids.contains(&p.pid));
        }
        
        // build output string
        let mut out = String::new();
        
        if let Some(fmt) = custom_format {
            // custom format mode
            let cols: Vec<&str> = fmt.split(',').collect();
            
            // header row
            for col in &cols {
                out.push_str(&format!("{:>8} ", col.to_uppercase()));
            }
            out.push('\n');
            
            // data rows
            for p in &procs {
                for col in &cols {
                    let val = match *col {
                        "pid" => p.pid.to_string(),
                        "ppid" => p.ppid.to_string(),
                        "user" => p.user.clone(),
                        "tty" => p.tty.clone(),
                        "cmd" | "command" | "args" => p.cmd.clone(),
                        "stat" | "state" => p.state.clone(),
                        _ => "?".to_string(),
                    };
                    out.push_str(&format!("{:>8} ", val));
                }
                out.push('\n');
            }
        } else if full {
            // full format
            out.push_str("  PID  PPID USER     TTY      STAT CMD\n");
            for p in &procs {
                out.push_str(&format!("{:5} {:5} {:<8} {:<8} {:<4} {}\n", p.pid, p.ppid, p.user, p.tty, p.state, p.cmd));
            }
        } else {
            // default format
            out.push_str("  PID TTY      STAT CMD\n");
            for p in &procs {
                out.push_str(&format!("{:5} {:<8} {:<4} {}\n", p.pid, p.tty, p.state, p.cmd));
            }
        }
        
        Ok(out)
    }
}

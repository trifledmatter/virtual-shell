use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

/// help [COMMAND]
/// Display help information about available commands.
pub struct HelpCommand;

const HELP_VERSION: &str = "help 1.0.0";
const HELP_USAGE: &str = "Usage: help [COMMAND]\nDisplay help information about available commands.\n\n  COMMAND        show help for specific command\n      --help     display this help and exit\n      --version  output version information and exit";

impl Command for HelpCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle simple flag cases first
        if args.iter().any(|a| a == "--help") {
            return Ok(HELP_USAGE.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(HELP_VERSION.to_string());
        }

        // single arg = help for specific command
        if let Some(cmd_name) = args.get(0) {
            // avoid infinite recursion - just show usage
            if cmd_name == "help" {
                return Ok(HELP_USAGE.to_string());
            }
            
            // make sure command actually exists
            let command_exists = if let Some(registry) = ctx.get_command_registry() {
                registry.get(cmd_name).is_some()
            } else {
                return Err("help: unable to access command registry".to_string());
            };
            
            if !command_exists {
                return Err(format!("help: no help topics match '{}'", cmd_name));
            }
            
            // grab help text for the requested command
            return Ok(get_command_help(cmd_name));
        }

        // no args = show all commands grouped by category
        if let Some(registry) = ctx.get_command_registry() {
            let command_names = registry.get_command_names();
            let mut output = String::from("Available commands:\n\n");
            
            // group by rough categories
            let mut file_ops = Vec::new();
            let mut system_ops = Vec::new();
            let mut text_ops = Vec::new();
            let mut env_ops = Vec::new();
            let mut other_ops = Vec::new();
            
            // sort cmds into categories - not perfect but good enough
            for cmd in &command_names {
                match cmd.as_str() {
                    "ls" | "mkdir" | "mk" | "touch" | "rm" | "rmdir" | "cp" | "mv" | 
                    "ln" | "chmod" | "chown" | "chgrp" | "pwd" | "cd" => {
                        file_ops.push(cmd);
                    }
                    "ps" | "kill" | "killall" | "cpu" => {
                        system_ops.push(cmd);
                    }
                    "cat" | "echo" | "grep" | "sed" | "edit" => {
                        text_ops.push(cmd);
                    }
                    "env" | "export" | "set" | "alias" | "unalias" | "source" | "functions" => {
                        env_ops.push(cmd);
                    }
                    "help" | "history" | "clear" | "rawcreate" => {
                        other_ops.push(cmd);
                    }
                    _ => {
                        other_ops.push(cmd);
                    }
                }
            }
            
            // print each category if not empty
            if !file_ops.is_empty() {
                output.push_str("File Operations:\n");
                for cmd in file_ops {
                    output.push_str(&format!("  {}\n", cmd));
                }
                output.push('\n');
            }
            
            if !text_ops.is_empty() {
                output.push_str("Text Operations:\n");
                for cmd in text_ops {
                    output.push_str(&format!("  {}\n", cmd));
                }
                output.push('\n');
            }
            
            if !system_ops.is_empty() {
                output.push_str("System Operations:\n");
                for cmd in system_ops {
                    output.push_str(&format!("  {}\n", cmd));
                }
                output.push('\n');
            }
            
            if !env_ops.is_empty() {
                output.push_str("Environment & Shell:\n");
                for cmd in env_ops {
                    output.push_str(&format!("  {}\n", cmd));
                }
                output.push('\n');
            }
            
            if !other_ops.is_empty() {
                output.push_str("Other Commands:\n");
                for cmd in other_ops {
                    output.push_str(&format!("  {}\n", cmd));
                }
                output.push('\n');
            }
            
            // add helpful footer
            output.push_str("Use 'help COMMAND' to get help for a specific command.\n");
            output.push_str("Most commands also support --help flag for detailed usage information.\n");
            
            Ok(output)
        } else {
            Err("help: unable to access command registry".to_string())
        }
    }
}

fn get_command_help(cmd_name: &str) -> String {
    match cmd_name {
        "ls" => "ls [OPTION]... [FILE]...\nList directory contents\n\nOptions:\n  -l        use a long listing format\n  -a        do not ignore entries starting with .\n  -h        with -l, print sizes in human readable format\n  --help    display this help and exit".to_string(),
        "mkdir" => "mkdir [OPTION]... DIRECTORY...\nCreate the DIRECTORY(ies), if they do not already exist\n\nOptions:\n  -p        make parent directories as needed\n  --help    display this help and exit".to_string(),
        "mk" => "mk FILE\nCreate an empty file\n\nOptions:\n  --help    display this help and exit".to_string(),
        "touch" => "touch [OPTION]... FILE...\nUpdate the access and modification times of each FILE to the current time\n\nOptions:\n  -a        change only the access time\n  -m        change only the modification time\n  --help    display this help and exit".to_string(),
        "echo" => "echo [STRING]...\nWrite arguments to the standard output\n\nOptions:\n  -n        do not output the trailing newline\n  --help    display this help and exit".to_string(),
        "pwd" => "pwd\nPrint the full filename of the current working directory\n\nOptions:\n  --help    display this help and exit".to_string(),
        "cd" => "cd [DIRECTORY]\nChange the current directory to DIRECTORY\n\nOptions:\n  --help    display this help and exit".to_string(),
        "cp" => "cp [OPTION]... SOURCE... DEST\nCopy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY\n\nOptions:\n  -r, -R    copy directories recursively\n  --help    display this help and exit".to_string(),
        "mv" => "mv [OPTION]... SOURCE... DEST\nMove/rename SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY\n\nOptions:\n  --help    display this help and exit".to_string(),
        "rm" => "rm [OPTION]... FILE...\nRemove (unlink) the FILE(s)\n\nOptions:\n  -r, -R    remove directories and their contents recursively\n  --help    display this help and exit".to_string(),
        "rmdir" => "rmdir [OPTION]... DIRECTORY...\nRemove empty directories\n\nOptions:\n  --help    display this help and exit".to_string(),
        "cat" => "cat [OPTION]... [FILE]...\nConcatenate FILE(s) and print on the standard output\n\nOptions:\n  --help    display this help and exit".to_string(),
        "grep" => "grep [OPTION]... PATTERN [FILE]...\nSearch for PATTERN in each FILE\n\nOptions:\n  -i        ignore case distinctions\n  --help    display this help and exit".to_string(),
        "sed" => "sed [OPTION]... SCRIPT [FILE]...\nStream editor for filtering and transforming text\n\nOptions:\n  --help    display this help and exit".to_string(),
        "chmod" => "chmod [OPTION]... MODE FILE...\nChange file mode bits\n\nOptions:\n  --help    display this help and exit".to_string(),
        "chown" => "chown [OPTION]... OWNER[:GROUP] FILE...\nChange file owner and group\n\nOptions:\n  --help    display this help and exit".to_string(),
        "chgrp" => "chgrp [OPTION]... GROUP FILE...\nChange group ownership\n\nOptions:\n  --help    display this help and exit".to_string(),
        "ln" => "ln [OPTION]... TARGET LINK_NAME\nCreate a link to TARGET with the name LINK_NAME\n\nOptions:\n  -s        make symbolic links instead of hard links\n  --help    display this help and exit".to_string(),
        "ps" => "ps [OPTION]...\nReport a snapshot of the current processes\n\nOptions:\n  -e, -A    select all processes\n  --help    display this help and exit".to_string(),
        "kill" => "kill [OPTION]... PID...\nTerminate processes by PID\n\nOptions:\n  -9        force kill\n  --help    display this help and exit".to_string(),
        "killall" => "killall [OPTION]... NAME...\nKill processes by name\n\nOptions:\n  --help    display this help and exit".to_string(),
        "env" => "env [OPTION]... [NAME=VALUE]... [COMMAND [ARG]...]\nRun a program in a modified environment\n\nOptions:\n  --help    display this help and exit".to_string(),
        "export" => "export [NAME[=VALUE]]...\nSet export attribute for shell variables\n\nOptions:\n  --help    display this help and exit".to_string(),
        "alias" => "alias [NAME[=VALUE]]...\nDefine or display aliases\n\nOptions:\n  --help    display this help and exit".to_string(),
        "unalias" => "unalias NAME...\nRemove each NAME from the list of defined aliases\n\nOptions:\n  --help    display this help and exit".to_string(),
        "set" => "set [OPTION]... [NAME[=VALUE]]...\nSet or unset values of shell options and positional parameters\n\nOptions:\n  -e        exit immediately if a command exits with a non-zero status\n  -x        print commands and their arguments as they are executed\n  --help    display this help and exit".to_string(),
        "source" => "source FILENAME [ARGUMENTS]\nRead and execute commands from FILENAME in the current shell environment\n\nOptions:\n  --help    display this help and exit".to_string(),
        "functions" => "functions\nDisplay all defined shell functions\n\nOptions:\n  --help    display this help and exit".to_string(),
        "history" => "history\nDisplay command history\n\nOptions:\n  --help    display this help and exit".to_string(),
        "cpu" => "cpu SUBCOMMAND [ARGS]...\nCPU emulator commands\n\nSubcommands:\n  run FILE      execute assembly file\n  debug FILE    debug assembly file step by step\n  template TYPE create assembly template\n  --help        display this help and exit".to_string(),
        "edit" => "edit FILE\nOpen FILE in nano-style editor\n\nOptions:\n  --help    display this help and exit".to_string(),
        "clear" => "clear\nClear the terminal screen\n\nOptions:\n  --help    display this help and exit".to_string(),
        "rawcreate" => "rawcreate <PATH> <HEX BYTES...>\ncreate a file with arbitrary bytes (hex)\n\nOptions:\n  --help    display this help and exit".to_string(),
        _ => format!("{} - No detailed help available\n\nTry running '{} --help' for more information.", cmd_name, cmd_name),
    }
}
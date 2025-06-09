use crate::context::TerminalContext;
use std::collections::HashMap;

pub type CommandResult = Result<String, String>;

pub trait Command {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult;
}

pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command + Send + Sync>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { commands: HashMap::new() }
    }
    pub fn register_command(&mut self, name: &str, cmd: Box<dyn Command + Send + Sync>) {
        self.commands.insert(name.to_string(), cmd);
    }
    pub fn get(&self, name: &str) -> Option<&Box<dyn Command + Send + Sync>> {
        self.commands.get(name)
    }
    pub fn get_command_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }
    pub fn default_commands() -> Self {
        let mut reg = Self::new();
        reg.register_command("ls", Box::new(crate::commands::ls::LsCommand));
        reg.register_command("mk", Box::new(crate::commands::mk::MkCommand));
        reg.register_command("mkdir", Box::new(crate::commands::mkdir::MkdirCommand));
        reg.register_command("touch", Box::new(crate::commands::touch::TouchCommand));
        reg.register_command("echo", Box::new(crate::commands::echo::EchoCommand));
        reg.register_command("pwd", Box::new(crate::commands::pwd::PwdCommand));
        reg.register_command("ln", Box::new(crate::commands::ln::LnCommand));
        reg.register_command("rmdir", Box::new(crate::commands::rmdir::RmdirCommand));
        reg.register_command("cp", Box::new(crate::commands::cp::CpCommand));
        reg.register_command("mv", Box::new(crate::commands::mv::MvCommand));
        reg.register_command("rm", Box::new(crate::commands::rm::RmCommand));
        reg.register_command("grep", Box::new(crate::commands::grep::GrepCommand));
        reg.register_command("sed", Box::new(crate::commands::sed::SedCommand));
        reg.register_command("chmod", Box::new(crate::commands::chmod::ChmodCommand));
        reg.register_command("chown", Box::new(crate::commands::chown::ChownCommand));
        reg.register_command("chgrp", Box::new(crate::commands::chgrp::ChgrpCommand));
        reg.register_command("ps", Box::new(crate::commands::ps::PsCommand));
        reg.register_command("kill", Box::new(crate::commands::kill::KillCommand));
        reg.register_command("killall", Box::new(crate::commands::killall::KillallCommand));
        reg.register_command("export", Box::new(crate::commands::export::ExportCommand));
        reg.register_command("env", Box::new(crate::commands::env::EnvCommand));
        reg.register_command("alias", Box::new(crate::commands::alias::AliasCommand));
        reg.register_command("unalias", Box::new(crate::commands::unalias::UnaliasCommand));
        reg.register_command("source", Box::new(crate::commands::source::SourceCommand));
        reg.register_command("set", Box::new(crate::commands::set::SetCommand));
        reg.register_command("functions", Box::new(crate::commands::functions::FunctionsCommand));
        reg.register_command("history", Box::new(crate::commands::history::HistoryCommand));
        reg.register_command("cpu", Box::new(crate::commands::cpu::CpuCommand));
        reg.register_command("edit", Box::new(crate::commands::edit::EditCommand));
        reg.register_command("edit_input", Box::new(crate::commands::edit::EditInputCommand));
        reg.register_command("cat", Box::new(crate::commands::cat::CatCommand));
        reg.register_command("cd", Box::new(crate::commands::cd::CdCommand));
        reg.register_command("help", Box::new(crate::commands::help::HelpCommand));
        reg
    }
}

pub fn run_command(input: &str, ctx: &mut TerminalContext, registry: &CommandRegistry) -> CommandResult {
    let input = input.trim();
    
    // special case for edit_input - need to keep spaces intact
    if input.starts_with("edit_input ") {
        let edit_args = &input[11..]; // chop off cmd prefix
        if let Some(command) = registry.get("edit_input") {
            // dump the whole thing as one arg, spaces and all
            return command.execute(&[edit_args.to_string()], ctx);
        }
    }
    
    // standard command handling for everything else
    let mut parts = input.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c,
        None => return Ok(String::new()), // empty input = no-op
    };
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();
    
    // find & run cmd or bail with err
    if let Some(command) = registry.get(cmd) {
        command.execute(&args, ctx)
    } else {
        Err(format!("Command not found: {}", cmd))
    }
}

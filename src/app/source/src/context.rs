use crate::vfs::VirtualFileSystem;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ShellOptions {
    pub errexit: bool, // set -e
    pub xtrace: bool,  // set -x
    // Add more options as needed
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            errexit: false,
            xtrace: false,
        }
    }
}

pub struct TerminalContext {
    pub vfs: VirtualFileSystem,
    pub env: HashMap<String, String>,
    pub vars: HashMap<String, String>, // shell-local variables
    pub cwd: String,
    pub aliases: HashMap<String, String>,
    pub args: Vec<String>, // positional parameters for scripts
    pub registry: Option<Arc<crate::command::CommandRegistry>>, // for script/source execution
    pub functions: HashMap<String, String>, // shell functions: name -> body
    pub options: ShellOptions, // shell options
    pub history: Vec<String>, // command history
}

impl TerminalContext {
    pub fn new() -> Self {
        let mut vfs = VirtualFileSystem::new();
        
        // Create essential directories
        let _ = vfs.create_dir("/home");
        let _ = vfs.create_dir("/tmp");
        let _ = vfs.create_dir("/usr");
        let _ = vfs.create_dir("/var");
        let _ = vfs.create_dir("/bin");
        let _ = vfs.create_dir("/etc");
        
        Self {
            vfs,
            env: HashMap::new(),
            vars: HashMap::new(),
            cwd: "/".to_string(),
            aliases: HashMap::new(),
            args: Vec::new(),
            registry: None,
            functions: HashMap::new(),
            options: ShellOptions::default(),
            history: Vec::new(),
        }
    }
    
    pub fn new_with_vfs(vfs: VirtualFileSystem) -> Self {
        Self {
            vfs,
            env: HashMap::new(),
            vars: HashMap::new(),
            cwd: "/".to_string(),
            aliases: HashMap::new(),
            args: Vec::new(),
            registry: None,
            functions: HashMap::new(),
            options: ShellOptions::default(),
            history: Vec::new(),
        }
    }
    
    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
    pub fn get_arg(&self, n: usize) -> Option<&String> {
        self.args.get(n)
    }
    pub fn define_function(&mut self, name: &str, body: &str) {
        self.functions.insert(name.to_string(), body.to_string());
    }
    pub fn get_function(&self, name: &str) -> Option<&String> {
        self.functions.get(name)
    }
    pub fn set_var(&mut self, name: &str, value: &str) {
        self.vars.insert(name.to_string(), value.to_string());
    }
    pub fn get_var(&self, name: &str) -> Option<&String> {
        self.vars.get(name)
    }
    pub fn set_option(&mut self, errexit: Option<bool>, xtrace: Option<bool>) {
        if let Some(e) = errexit { self.options.errexit = e; }
        if let Some(x) = xtrace { self.options.xtrace = x; }
    }
    
    pub fn get_command_registry(&self) -> Option<&Arc<crate::command::CommandRegistry>> {
        self.registry.as_ref()
    }
    
    pub fn set_command_registry(&mut self, registry: Arc<crate::command::CommandRegistry>) {
        self.registry = Some(registry);
    }
}

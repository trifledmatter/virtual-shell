use crate::vfs::VirtualFileSystem;
use crate::vfs_events::emit_vfs_event;
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
    
    /// Create a file with VFS event emission
    pub fn create_file_with_events(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        web_sys::console::log_3(
            &"[CONTEXT VFS] 📝 create_file_with_events called for:".into(),
            &path.into(),
            &format!("({} bytes)", content.len()).into(),
        );
        
        // Create the file
        match self.vfs.create_file(path, content.to_vec()) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ File created, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-create-file", path, Some(content));
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[CONTEXT VFS] ❌ Failed to create file:".into(),
                    &path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }
    
    /// Write to a file with VFS event emission
    pub fn write_file_with_events(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        web_sys::console::log_3(
            &"[CONTEXT VFS] 📝 write_file_with_events called for:".into(),
            &path.into(),
            &format!("({} bytes)", content.len()).into(),
        );

        // Try write first, then create if needed
        match self.vfs.write_file(path, content.to_vec()) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ File written, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-write-file", path, Some(content));
                Ok(())
            }
            Err(_) => {
                web_sys::console::log_1(&"[CONTEXT VFS] 📝 File doesn't exist, creating with events".into());
                // File doesn't exist, create it with events
                self.create_file_with_events(path, content)
            }
        }
    }
    
    /// Create a symlink with VFS event emission
    pub fn create_symlink_with_events(&mut self, link_path: &str, target_path: &str) -> Result<(), String> {
        web_sys::console::log_3(
            &"[CONTEXT VFS] 🔗 create_symlink_with_events called for:".into(),
            &link_path.into(),
            &format!("-> {}", target_path).into(),
        );
        
        match self.vfs.create_symlink(link_path, target_path) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ Symlink created, emitting VFS event:".into(),
                    &link_path.into(),
                );
                // Emit VFS event for frontend (no content for symlinks, just the path)
                emit_vfs_event("vfs-create-symlink", link_path, None);
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[CONTEXT VFS] ❌ Failed to create symlink:".into(),
                    &link_path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }
    
    /// Create a directory with VFS event emission
    pub fn create_dir_with_events(&mut self, path: &str) -> Result<(), String> {
        web_sys::console::log_2(
            &"[CONTEXT VFS] 📁 create_dir_with_events called for:".into(),
            &path.into(),
        );
        
        match self.vfs.create_dir(path) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ Directory created, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend
                emit_vfs_event("vfs-create-dir", path, None);
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[CONTEXT VFS] ❌ Failed to create directory:".into(),
                    &path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }
    
    /// Create a zip archive with VFS event emission
    pub fn create_zip_with_events(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        web_sys::console::log_3(
            &"[CONTEXT VFS] 📦 create_zip_with_events called for:".into(),
            &path.into(),
            &format!("({} bytes)", content.len()).into(),
        );
        
        // Create the zip file
        match self.vfs.create_file(path, content.to_vec()) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ Zip archive created, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit specific VFS event for zip archives
                emit_vfs_event("vfs-create-zip", path, Some(content));
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[CONTEXT VFS] ❌ Failed to create zip archive:".into(),
                    &path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }
    
    /// Delete a file or directory with VFS event emission
    pub fn delete_with_events(&mut self, path: &str) -> Result<(), String> {
        web_sys::console::log_2(
            &"[CONTEXT VFS] 🗑️ delete_with_events called for:".into(),
            &path.into(),
        );
        
        match self.vfs.delete(path) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[CONTEXT VFS] ✅ File/directory deleted, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend
                emit_vfs_event("vfs-delete", path, None);
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[CONTEXT VFS] ❌ Failed to delete:".into(),
                    &path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }
}

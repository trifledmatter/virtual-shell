// wasm terminal emulator with nano editor
// basically a fake shell that runs in the browser
pub mod vfs;
pub mod command;
pub mod context;
pub mod commands;
pub mod vfs_events;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;
use context::TerminalContext;
use command::{CommandRegistry};
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};
use web_sys::{window, CustomEvent, CustomEventInit};
use vfs_events::emit_vfs_event;

// better errors in browser console
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// Global callback for async results - will be set by JavaScript
static mut ASYNC_CALLBACK: Option<js_sys::Function> = None;

// Function to set the async callback from JavaScript
#[wasm_bindgen]
pub fn set_async_result_callback(callback: js_sys::Function) {
    unsafe {
        ASYNC_CALLBACK = Some(callback);
    }
}

// Function to send async results to the terminal
pub fn send_async_result(result: &str) {
    unsafe {
        if let Some(ref callback) = ASYNC_CALLBACK {
            let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(result));
        }
    }
}



// main terminal struct - keeps state between calls
// ctx = context, registry = available commands
#[wasm_bindgen]
pub struct Terminal {
    ctx: TerminalContext,
    registry: CommandRegistry,
}

// response wrapper for js comms
// just success flag + output string
#[derive(Serialize, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_action: Option<String>,
}

#[wasm_bindgen]
impl Terminal {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Terminal {
        // setup default commands and context
        let registry = CommandRegistry::default_commands();
        let mut ctx = TerminalContext::new();
        
        // enable auto-save by default
        ctx.set_var("_auto_save", "true");
        
        // need separate registry for ctx since can't clone it
        // rust ownership nonsense, whatever
        let registry_for_ctx = CommandRegistry::default_commands();
        ctx.set_command_registry(std::sync::Arc::new(registry_for_ctx));
        
        Terminal {
            ctx,
            registry,
        }
    }

    /// initialize terminal - call this immediately after creating terminal
    #[wasm_bindgen]
    pub async fn init_terminal(&mut self) -> JsValue {
        // no saved data or error loading, use fresh vfs
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "message": "terminal initialized with fresh filesystem"
        })).unwrap()
    }

    /// initialize terminal with storage support - frontend compatibility method
    #[wasm_bindgen]
    pub async fn init_with_storage(&mut self) -> JsValue {
        // Initialize with persistent storage support
        // The actual persistence is handled by the frontend ZenFS system
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "message": "terminal initialized with persistent storage"
        })).unwrap()
    }

    /// load filesystem data from frontend (ZenFS)
    #[wasm_bindgen]
    pub fn load_filesystem_data(&mut self, files_json: &str) -> JsValue {
        web_sys::console::log_1(&"[RUST VFS] 📥 Loading filesystem data from ZenFS...".into());
        
        match serde_json::from_str::<Vec<serde_json::Value>>(files_json) {
            Ok(files) => {
                web_sys::console::log_2(
                    &"[RUST VFS] 📊 Processing".into(),
                    &(files.len() as u32).into(),
                );
                web_sys::console::log_1(&"files".into());
                
                let mut loaded_count = 0;
                let mut error_count = 0;

                for file_data in files {
                    if let (Some(path), Some(content_array)) = (
                        file_data.get("path").and_then(|p| p.as_str()),
                        file_data.get("content").and_then(|c| c.as_array())
                    ) {
                        // Convert JSON array to bytes
                        let content: Result<Vec<u8>, _> = content_array
                            .iter()
                            .map(|v| v.as_u64().map(|n| n as u8))
                            .collect::<Option<Vec<_>>>()
                            .ok_or("Invalid content data");

                        match content {
                            Ok(content_bytes) => {
                                // Create directories as needed
                                if let Some(parent_dir) = std::path::Path::new(path).parent() {
                                    let parent_str = parent_dir.to_string_lossy();
                                    if parent_str != "/" && !parent_str.is_empty() {
                                        let _ = self.create_directories_recursive(&parent_str);
                                    }
                                }

                                // Create the file
                                web_sys::console::log_3(
                                    &"[RUST VFS] 📝 Creating file:".into(),
                                    &path.into(),
                                    &format!("({} bytes)", content_bytes.len()).into(),
                                );
                                
                                match self.ctx.vfs.create_file(path, content_bytes.clone()) {
                                    Ok(_) => {
                                        web_sys::console::log_2(
                                            &"[RUST VFS] ✅ File created:".into(),
                                            &path.into(),
                                        );
                                        loaded_count += 1;
                                    }
                                    Err(_) => {
                                        web_sys::console::log_2(
                                            &"[RUST VFS] 🔄 File exists, updating:".into(),
                                            &path.into(),
                                        );
                                        // File might already exist, try to update it
                                        match self.ctx.vfs.write_file(path, content_bytes) {
                                            Ok(_) => {
                                                web_sys::console::log_2(
                                                    &"[RUST VFS] ✅ File updated:".into(),
                                                    &path.into(),
                                                );
                                                loaded_count += 1;
                                            }
                                            Err(e) => {
                                                web_sys::console::error_3(
                                                    &"[RUST VFS] ❌ Failed to update file:".into(),
                                                    &path.into(),
                                                    &e.into(),
                                                );
                                                error_count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_) => error_count += 1,
                        }
                    } else {
                        error_count += 1;
                    }
                }

                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": true,
                    "loaded": loaded_count,
                    "errors": error_count,
                    "message": format!("Loaded {} files from persistent storage", loaded_count)
                })).unwrap()
            }
            Err(e) => {
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": format!("Failed to parse filesystem data: {}", e)
                })).unwrap()
            }
        }
    }

    /// helper to create directories recursively
    fn create_directories_recursive(&mut self, path: &str) -> Result<(), String> {
        let components: Vec<&str> = path.trim_matches('/').split('/').filter(|c| !c.is_empty()).collect();
        let mut current_path = String::new();

        for component in components {
            current_path = if current_path.is_empty() {
                format!("/{}", component)
            } else {
                format!("{}/{}", current_path, component)
            };

            // Try to create directory, ignore if it already exists
            let _ = self.ctx.vfs.create_dir(&current_path);
        }

        Ok(())
    }

    /// helper to create files with automatic VFS event emission
    pub fn create_file_with_events(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        web_sys::console::log_3(
            &"[RUST VFS] 📝 create_file_with_events called for:".into(),
            &path.into(),
            &format!("({} bytes)", content.len()).into(),
        );
        
        // Create directories as needed
        if let Some(parent_dir) = std::path::Path::new(path).parent() {
            let parent_str = parent_dir.to_string_lossy();
            if parent_str != "/" && !parent_str.is_empty() {
                let _ = self.create_directories_recursive(&parent_str);
            }
        }

        // Create the file
        match self.ctx.vfs.create_file(path, content.to_vec()) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[RUST VFS] ✅ File created, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-create-file", path, Some(content));
                Ok(())
            }
            Err(e) => {
                web_sys::console::error_3(
                    &"[RUST VFS] ❌ Failed to create file:".into(),
                    &path.into(),
                    &e.clone().into(),
                );
                Err(e)
            }
        }
    }

    /// helper to write files with automatic VFS event emission  
    pub fn write_file_with_events(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        web_sys::console::log_3(
            &"[RUST VFS] 📝 write_file_with_events called for:".into(),
            &path.into(),
            &format!("({} bytes)", content.len()).into(),
        );

        // Try write first, then create if needed
        match self.ctx.vfs.write_file(path, content.to_vec()) {
            Ok(_) => {
                web_sys::console::log_2(
                    &"[RUST VFS] ✅ File written, emitting VFS event:".into(),
                    &path.into(),
                );
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-write-file", path, Some(content));
                Ok(())
            }
            Err(_) => {
                web_sys::console::log_1(&"[RUST VFS] 📝 File doesn't exist, creating with events".into());
                // File doesn't exist, create it with events
                self.create_file_with_events(path, content)
            }
        }
    }

    /// test function to manually emit a VFS event - for debugging
    #[wasm_bindgen]
    pub fn test_emit_event(&self) -> JsValue {
        web_sys::console::log_1(&"[RUST VFS] 🧪 Manually testing event emission...".into());
        
        // Check if we have access to window and document
        if let Some(win) = window() {
            web_sys::console::log_1(&"[RUST VFS] ✅ Window object available".into());
            
            // Check for global VFS callback
            let global = win.as_ref();
            if let Ok(callback_prop) = js_sys::Reflect::get(global, &"__vfsCallback".into()) {
                if !callback_prop.is_undefined() && callback_prop.is_function() {
                    web_sys::console::log_1(&"[RUST VFS] ✅ Global __vfsCallback found and is a function".into());
                } else {
                    web_sys::console::warn_1(&"[RUST VFS] ⚠️ __vfsCallback found but is not a function".into());
                }
            } else {
                web_sys::console::warn_1(&"[RUST VFS] ⚠️ __vfsCallback not found on window".into());
            }
            
            if let Some(doc) = win.document() {
                web_sys::console::log_1(&"[RUST VFS] ✅ Document object available".into());
            } else {
                web_sys::console::warn_1(&"[RUST VFS] ⚠️ Document object not available".into());
            }
            
            // Check if we can access the location
            let location = win.location();
            if let Ok(href) = location.href() {
                web_sys::console::log_2(&"[RUST VFS] 📍 Page location:".into(), &href.into());
            }
        } else {
            web_sys::console::warn_1(&"[RUST VFS] ⚠️ Window object not available".into());
        }
        
        emit_vfs_event("vfs-write-file", "/test-from-rust.txt", Some(b"Hello from Rust!"));
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "message": "Test event emitted"
        })).unwrap()
    }

    // main entry point - run a command and return result
    #[wasm_bindgen]
    pub fn execute_command(&mut self, input: &str) -> JsValue {
        let input = input.trim();
        
        let response = match command::run_command(input, &mut self.ctx, &self.registry) {
            Ok(output) => {
                // Check for special action markers
                let cmd_response = match output.as_str() {
                    "__CLEAR_SCREEN__" => CommandResponse {
                        success: true,
                        output: "".to_string(),
                        special_action: Some("clear_screen".to_string()),
                    },
                    _ => CommandResponse {
                        success: true,
                        output,
                        special_action: None,
                    },
                };

                cmd_response
            },
            Err(e) => CommandResponse {
                success: false,
                output: format!("Error: {}", e),
                special_action: None,
            },
        };

        serde_wasm_bindgen::to_value(&response).unwrap()
    }

    // get current working directory
    #[wasm_bindgen]
    pub fn get_current_directory(&self) -> String {
        self.ctx.cwd.clone()
    }

    // list files in a directory
    // returns json with file info
    #[wasm_bindgen]
    pub fn list_files(&self, path: Option<String>) -> JsValue {
        let target_path = path.unwrap_or_else(|| self.ctx.cwd.clone());
        
        match self.ctx.vfs.list_dir(&target_path) {
            Ok(entries) => {
                let mut files: Vec<serde_json::Value> = Vec::new();
                
                for entry_name in entries {
                    // build full path
                    let full_path = if target_path == "/" {
                        format!("/{}", entry_name)
                    } else {
                        format!("{}/{}", target_path, entry_name)
                    };
                    
                    // get file metadata
                    if let Some(node) = self.ctx.vfs.resolve_path(&full_path) {
                        let (is_directory, size) = match node {
                            crate::vfs::VfsNode::Directory { .. } => (true, 0),
                            crate::vfs::VfsNode::File { content, .. } => (false, content.len()),
                            crate::vfs::VfsNode::Symlink { .. } => (false, 0),
                        };
                        
                        files.push(serde_json::json!({
                            "name": entry_name,
                            "type": if is_directory { "directory" } else { "file" },
                            "size": size,
                        }));
                    }
                }
                
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": true,
                    "files": files,
                })).unwrap()
            }
            Err(e) => {
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })).unwrap()
            }
        }
    }

    // read file contents as utf8 string
    #[wasm_bindgen]
    pub fn read_file(&self, path: &str) -> JsValue {
        // handle relative/absolute paths
        let full_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.ctx.cwd, path)
        };
        
        match self.ctx.vfs.read_file(&full_path) {
            Ok(content_bytes) => {
                // try to convert to utf8
                match String::from_utf8(content_bytes.to_vec()) {
                    Ok(content) => {
                        serde_wasm_bindgen::to_value(&serde_json::json!({
                            "success": true,
                            "content": content,
                        })).unwrap()
                    }
                    Err(_) => {
                        serde_wasm_bindgen::to_value(&serde_json::json!({
                            "success": false,
                            "error": "File contains invalid UTF-8",
                        })).unwrap()
                    }
                }
            }
            Err(e) => {
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })).unwrap()
            }
        }
    }

    // write string to file, create if doesn't exist
    #[wasm_bindgen]
    pub async fn write_file(&mut self, path: &str, content: &str) -> JsValue {
        web_sys::console::log_3(
            &"[RUST VFS] 📝 write_file called for:".into(),
            &path.into(),
            &format!("({} chars)", content.len()).into(),
        );
        
        // handle relative/absolute paths
        let full_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.ctx.cwd, path)
        };
        
        web_sys::console::log_2(
            &"[RUST VFS] 📍 Full path resolved to:".into(),
            &full_path.clone().into(),
        );
        
        // try write first, then create if needed
        let result = match self.ctx.vfs.write_file(&full_path, content.as_bytes().to_vec()) {
            Ok(_) => {
                web_sys::console::log_1(&"[RUST VFS] ✅ File written to VFS successfully".into());
                Ok(())
            }
            Err(_) => {
                web_sys::console::log_1(&"[RUST VFS] 📝 File doesn't exist, creating new file".into());
                // file doesn't exist, create it
                self.ctx.vfs.create_file(&full_path, content.as_bytes().to_vec())
            }
        };
        
        match result {
            Ok(_) => {
                web_sys::console::log_1(&"[RUST VFS] 🎯 About to emit VFS event...".into());
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-write-file", &full_path, Some(content.as_bytes()));
                
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": true,
                    "auto_saved": true,
                })).unwrap()
            }
            Err(e) => {
                web_sys::console::error_2(
                    &"[RUST VFS] ❌ Failed to write file:".into(),
                    &e.clone().into(),
                );
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })).unwrap()
            }
        }
    }

    // get list of available commands
    #[wasm_bindgen]
    pub fn get_command_list(&self) -> JsValue {
        let commands = self.registry.get_command_names();
        
        serde_wasm_bindgen::to_value(&commands).unwrap()
    }

    // get all env vars as json
    #[wasm_bindgen]
    pub fn get_environment_variables(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.ctx.env).unwrap()
    }

    // set env var
    #[wasm_bindgen]
    pub fn set_environment_variable(&mut self, key: &str, value: &str) {
        self.ctx.env.insert(key.to_string(), value.to_string());
    }
    
    // check if we're in nano edit mode
    #[wasm_bindgen]
    pub fn is_nano_mode(&self) -> bool {
        self.ctx.get_var("_nano_mode")
            .map(|s| s == "edit")
            .unwrap_or(false)
    }
    
    // get filename being edited in nano
    #[wasm_bindgen]
    pub fn get_nano_filename(&self) -> Option<String> {
        self.ctx.get_var("_nano_file").map(|s| s.clone())
    }
    
    // handle nano editor input (keyboard events or text)
    #[wasm_bindgen]
    pub fn process_nano_input(&mut self, input: &str) -> JsValue {
        if !self.is_nano_mode() {
            return serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": false,
                "error": "Not in nano edit mode"
            })).unwrap();
        }
        
        let filename = match self.get_nano_filename() {
            Some(f) => f,
            None => {
                return serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": "No file being edited"
                })).unwrap();
            }
        };
        
        // try to parse as json event first, fallback to plain text
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(input) {
            self.handle_nano_event(&event, &filename)
        } else {
            // plain text input
            self.handle_nano_text_input(input, &filename)
        }
    }
    
    // handle structured keyboard events
    fn handle_nano_event(&mut self, event: &serde_json::Value, filename: &str) -> JsValue {
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        
        match event_type {
            "click" => {
                // handle cursor positioning via mouse click
                let line = event.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let col = event.get("col").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                
                // get buffer to validate positions
                let buffer = self.ctx.get_var("_nano_buffer")
                    .map(|s| s.clone())
                    .unwrap_or_else(|| String::new());
                
                let lines: Vec<&str> = if buffer.is_empty() {
                    vec![""]
                } else {
                    buffer.lines().collect()
                };
                
                // clamp to valid range - don't let cursor go oob
                let target_line = line.min(lines.len().saturating_sub(1));
                let target_col = if target_line < lines.len() {
                    col.min(lines[target_line].len())
                } else {
                    0
                };
                
                // update cursor position
                self.ctx.set_var("_nano_cursor_line", &target_line.to_string());
                self.ctx.set_var("_nano_cursor_col", &target_col.to_string());
                
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": true,
                    "refresh": true
                })).unwrap()
            }
            "keydown" => {
                // handle keyboard shortcuts and regular keys
                let key = event.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let ctrl = event.get("ctrlKey").and_then(|v| v.as_bool()).unwrap_or(false);
                let _shift = event.get("shiftKey").and_then(|v| v.as_bool()).unwrap_or(false);
                
                // nano keyboard shortcuts
                match (ctrl, key) {
                    (true, "s") => self.nano_save_file(filename),
                    (true, "x") => self.nano_exit_editor(filename),
                    (true, "k") => self.nano_cut_line(),
                    (true, "u") => self.nano_paste_line(),
                    (false, "ArrowUp") => self.nano_move_cursor("up"),
                    (false, "ArrowDown") => self.nano_move_cursor("down"),
                    (false, "ArrowLeft") => self.nano_move_cursor("left"),
                    (false, "ArrowRight") => self.nano_move_cursor("right"),
                    (false, "Home") => self.nano_move_cursor("home"),
                    (false, "End") => self.nano_move_cursor("end"),
                    (false, "PageUp") => self.nano_move_cursor("pageup"),
                    (false, "PageDown") => self.nano_move_cursor("pagedown"),
                    (false, "Enter") => self.nano_insert_newline(),
                    (false, "Backspace") => self.nano_backspace(),
                    (false, "Delete") => self.nano_delete(),
                    _ => {
                        // regular character input
                        if let Some(char_input) = event.get("char").and_then(|v| v.as_str()) {
                            if !ctrl && char_input.len() == 1 {
                                self.nano_insert_char(char_input)
                            } else {
                                self.nano_no_action()
                            }
                        } else {
                            self.nano_no_action()
                        }
                    }
                }
            }
            _ => self.nano_no_action()
        }
    }
    
    // handle plain text input (legacy vim-style commands)
    fn handle_nano_text_input(&mut self, input: &str, filename: &str) -> JsValue {
        // handle old-school vim-style commands for backwards compat
        match input {
            ":w" | ":save" => self.nano_save_file(filename),
            ":q" | ":quit" => self.nano_exit_editor(filename),
            ":wq" => {
                self.nano_save_file(filename);
                self.nano_exit_editor(filename)
            }
            _ => {
                // just insert text at cursor
                self.nano_insert_text(input)
            }
        }
    }
    
    // save buffer to file
    fn nano_save_file(&mut self, filename: &str) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        // try to write, create if doesn't exist
        let result = self.ctx.vfs.write_file(filename, buffer.as_bytes().to_vec())
            .or_else(|_| self.ctx.vfs.create_file(filename, buffer.as_bytes().to_vec()));
        
        match result {
            Ok(_) => {
                // Emit VFS event for frontend to save to IndexedDB
                emit_vfs_event("vfs-write-file", filename, Some(buffer.as_bytes()));
                
                self.ctx.set_var("_nano_modified", "false");
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": true,
                    "message": format!("Wrote {} lines to {}", buffer.lines().count(), filename),
                    "refresh": true,
                    "auto_saved": true,
                })).unwrap()
            }
            Err(e) => {
                serde_wasm_bindgen::to_value(&serde_json::json!({
                    "success": false,
                    "error": format!("Error writing {}: {}", filename, e),
                    "refresh": false
                })).unwrap()
            }
        }
    }
    
    // exit nano editor, prompt to save if modified
    fn nano_exit_editor(&mut self, filename: &str) -> JsValue {
        let modified = self.ctx.get_var("_nano_modified")
            .map(|s| s == "true")
            .unwrap_or(false);
        
        if modified {
            // ask user if they want to save
            serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": true,
                "prompt_save": true,
                "message": format!("Save modified buffer to {}? (y/n)", filename)
            })).unwrap()
        } else {
            // clean exit, clear nano state
            self.ctx.set_var("_nano_mode", "");
            self.ctx.set_var("_nano_file", "");
            self.ctx.set_var("_nano_buffer", "");
            serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": true,
                "exit": true,
                "message": "Exited nano"
            })).unwrap()
        }
    }
    
    // move cursor around the buffer
    fn nano_move_cursor(&mut self, direction: &str) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let lines: Vec<&str> = if buffer.is_empty() {
            vec![""]
        } else {
            buffer.lines().collect()
        };
        
        let mut cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        // move cursor based on direction
        match direction {
            "up" => {
                if cursor_line > 0 {
                    cursor_line -= 1;
                    // clamp column to line length
                    cursor_col = cursor_col.min(lines.get(cursor_line).unwrap_or(&"").len());
                }
            }
            "down" => {
                if cursor_line < lines.len().saturating_sub(1) {
                    cursor_line += 1;
                    cursor_col = cursor_col.min(lines.get(cursor_line).unwrap_or(&"").len());
                }
            }
            "left" => {
                if cursor_col > 0 {
                    cursor_col -= 1;
                } else if cursor_line > 0 {
                    // wrap to end of previous line
                    cursor_line -= 1;
                    cursor_col = lines.get(cursor_line).unwrap_or(&"").len();
                }
            }
            "right" => {
                let current_line = lines.get(cursor_line).unwrap_or(&"");
                if cursor_col < current_line.len() {
                    cursor_col += 1;
                } else if cursor_line < lines.len().saturating_sub(1) {
                    // wrap to start of next line
                    cursor_line += 1;
                    cursor_col = 0;
                }
            }
            "home" => cursor_col = 0,
            "end" => cursor_col = lines.get(cursor_line).unwrap_or(&"").len(),
            "pageup" => cursor_line = cursor_line.saturating_sub(10),
            "pagedown" => cursor_line = (cursor_line + 10).min(lines.len().saturating_sub(1)),
            _ => {} // ignore unknown directions
        }
        
        // save new cursor position
        self.ctx.set_var("_nano_cursor_line", &cursor_line.to_string());
        self.ctx.set_var("_nano_cursor_col", &cursor_col.to_string());
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // insert single character at cursor
    fn nano_insert_char(&mut self, ch: &str) -> JsValue {
        println!("nano_insert_char called with: '{}'", ch);
        
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        println!("Current buffer: '{}'", buffer);
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        println!("Current cursor: line={}, col={}", cursor_line, cursor_col);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        // ensure buffer has enough lines
        while lines.len() <= cursor_line {
            lines.push(String::new());
        }
        
        // insert char at cursor position
        let line = &mut lines[cursor_line];
        if cursor_col <= line.len() {
            line.insert_str(cursor_col, ch);
            
            println!("Line after insertion: '{}'", line);
            
            // move cursor right and mark as modified
            self.ctx.set_var("_nano_cursor_col", &(cursor_col + 1).to_string());
            self.ctx.set_var("_nano_modified", "true");
            
            let new_buffer = lines.join("\n");
            self.ctx.set_var("_nano_buffer", &new_buffer);
            
            println!("New buffer: '{}'", new_buffer);
            println!("New cursor position: line={}, col={}", cursor_line, cursor_col + 1);
            
            serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": true,
                "refresh": true
            })).unwrap()
        } else {
            println!("Invalid cursor position: col={}, line_len={}", cursor_col, line.len());
            self.nano_no_action()
        }
    }
    
    // insert multiple characters
    fn nano_insert_text(&mut self, text: &str) -> JsValue {
        // just call insert_char for each character
        for ch in text.chars() {
            self.nano_insert_char(&ch.to_string());
        }
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // insert newline and split current line
    fn nano_insert_newline(&mut self) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        // ensure buffer has enough lines
        while lines.len() <= cursor_line {
            lines.push(String::new());
        }
        
        // split line at cursor - get parts before modifying
        let current_line = lines[cursor_line].clone();
        let cursor_pos = cursor_col.min(current_line.len());
        let left = current_line[..cursor_pos].to_string();
        let right = current_line[cursor_pos..].to_string();
        
        // split the line
        lines[cursor_line] = left;
        lines.insert(cursor_line + 1, right);
        
        // move cursor to start of new line
        self.ctx.set_var("_nano_cursor_line", &(cursor_line + 1).to_string());
        self.ctx.set_var("_nano_cursor_col", "0");
        self.ctx.set_var("_nano_modified", "true");
        
        let new_buffer = lines.join("\n");
        self.ctx.set_var("_nano_buffer", &new_buffer);
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // backspace - delete char before cursor
    fn nano_backspace(&mut self) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        if cursor_col > 0 {
            // delete char before cursor on same line
            let line = &mut lines[cursor_line];
            if cursor_col <= line.len() {
                line.remove(cursor_col - 1);
                self.ctx.set_var("_nano_cursor_col", &(cursor_col - 1).to_string());
                self.ctx.set_var("_nano_modified", "true");
            }
        } else if cursor_line > 0 {
            // join with previous line (delete newline)
            let current_line = lines.remove(cursor_line);
            let prev_line_len = lines[cursor_line - 1].len();
            lines[cursor_line - 1].push_str(&current_line);
            
            self.ctx.set_var("_nano_cursor_line", &(cursor_line - 1).to_string());
            self.ctx.set_var("_nano_cursor_col", &prev_line_len.to_string());
            self.ctx.set_var("_nano_modified", "true");
        }
        
        let new_buffer = lines.join("\n");
        self.ctx.set_var("_nano_buffer", &new_buffer);
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // delete - delete char at cursor
    fn nano_delete(&mut self) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        if cursor_line < lines.len() {
            let line = &mut lines[cursor_line];
            if cursor_col < line.len() {
                // delete char at cursor
                line.remove(cursor_col);
                self.ctx.set_var("_nano_modified", "true");
            } else if cursor_line < lines.len() - 1 {
                // join with next line (delete newline)
                let next_line = lines.remove(cursor_line + 1);
                lines[cursor_line].push_str(&next_line);
                self.ctx.set_var("_nano_modified", "true");
            }
        }
        
        let new_buffer = lines.join("\n");
        self.ctx.set_var("_nano_buffer", &new_buffer);
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // cut entire line to clipboard
    fn nano_cut_line(&mut self) -> JsValue {
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        if cursor_line < lines.len() {
            let cut_line = lines.remove(cursor_line);
            self.ctx.set_var("_nano_clipboard", &cut_line);
            self.ctx.set_var("_nano_modified", "true");
            
            // need at least one line in buffer
            if lines.is_empty() {
                lines.push(String::new());
            }
            
            // fix cursor if we deleted last line
            if cursor_line >= lines.len() {
                self.ctx.set_var("_nano_cursor_line", &(lines.len() - 1).to_string());
            }
            // reset column position to start of line
            self.ctx.set_var("_nano_cursor_col", "0");
            
            // update buffer after cut
            let new_buffer = lines.join("\n");
            self.ctx.set_var("_nano_buffer", &new_buffer);
        }
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // paste line from clipboard
    fn nano_paste_line(&mut self) -> JsValue {
        let clipboard = self.ctx.get_var("_nano_clipboard")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        if clipboard.is_empty() {
            return self.nano_no_action();
        }
        
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let mut lines: Vec<String> = if buffer.is_empty() {
            vec![String::new()]
        } else {
            buffer.lines().map(|s| s.to_string()).collect()
        };
        
        // insert clipboard content after current line
        lines.insert(cursor_line + 1, clipboard);
        self.ctx.set_var("_nano_cursor_line", &(cursor_line + 1).to_string());
        self.ctx.set_var("_nano_cursor_col", "0");
        self.ctx.set_var("_nano_modified", "true");
        
        let new_buffer = lines.join("\n");
        self.ctx.set_var("_nano_buffer", &new_buffer);
        
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // do nothing - for unsupported keys
    fn nano_no_action(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "success": true,
            "refresh": true
        })).unwrap()
    }
    
    // get complete nano editor state for frontend display
    #[wasm_bindgen]
    pub fn get_nano_editor_state(&self) -> JsValue {
        println!("🔍 get_nano_editor_state called");
        
        if !self.is_nano_mode() {
            println!("❌ Not in nano mode");
            return serde_wasm_bindgen::to_value(&serde_json::json!({
                "success": false,
                "error": "Not in nano mode"
            })).unwrap();
        }
        
        let filename = self.get_nano_filename().unwrap_or_default();
        println!("📁 Filename: {}", filename);
        
        let buffer = self.ctx.get_var("_nano_buffer")
            .map(|s| s.clone())
            .unwrap_or_else(|| String::new());
        
        println!("📄 Current buffer: '{}'", buffer);
        
        // split buffer into lines for display
        let lines: Vec<&str> = if buffer.is_empty() {
            vec![""]
        } else {
            buffer.lines().collect()
        };
        
        let cursor_line = self.ctx.get_var("_nano_cursor_line")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let cursor_col = self.ctx.get_var("_nano_cursor_col")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        let modified = self.ctx.get_var("_nano_modified")
            .map(|s| s == "true")
            .unwrap_or(false);
        
        println!("📍 Cursor: line={}, col={}", cursor_line, cursor_col);
        println!("✏️ Modified: {}", modified);
        println!("📝 Lines count: {}", lines.len());
        
        // figure out file type for syntax highlighting
        let file_type = if filename.ends_with(".asm") {
            "assembly"
        } else if filename.ends_with(".sh") || filename.ends_with(".bash") {
            "shell"
        } else if filename.ends_with(".md") {
            "markdown"
        } else {
            "text"
        };
        
        // build the complete editor state
        let editor_data = serde_json::json!({
            "success": true,
            "editor": {
                "type": "nano_editor",
                "filename": filename,
                "modified": modified,
                "lines": lines.iter().enumerate().map(|(i, line)| {
                    serde_json::json!({
                        "number": i + 1,
                        "content": line,
                        "current": i == cursor_line,
                        "syntax": self.get_syntax_highlights_for_line(line, file_type)
                    })
                }).collect::<Vec<_>>(),
                "cursor": {
                    "line": cursor_line,
                    "col": cursor_col
                },
                "status": format!("GNU nano  {}  {}", 
                    filename, 
                    if modified { "Modified" } else { "" }
                ),
                "help": "^S Save  ^X Exit  ^K Cut  ^U Paste  ^G Help"
            }
        });
        
        println!("📦 Returning editor data: {}", serde_json::to_string_pretty(&editor_data).unwrap_or_default());
        
        serde_wasm_bindgen::to_value(&editor_data).unwrap()
    }
    
    // basic syntax highlighting for different file types
    fn get_syntax_highlights_for_line(&self, line: &str, file_type: &str) -> Vec<serde_json::Value> {
        let mut highlights = Vec::new();
        
        match file_type {
            "assembly" => {
                // basic assembly syntax highlighting
                let instructions = ["push", "pop", "add", "sub", "mul", "div", "mod", 
                                   "dup", "swap", "load", "store", "jump", "jumpif", 
                                   "jumpifz", "cmp", "print", "printchar", "read", "halt"];
                
                let words: Vec<&str> = line.split_whitespace().collect();
                let mut pos = 0;
                
                for (i, word) in words.iter().enumerate() {
                    let start = line[pos..].find(word).unwrap_or(0) + pos;
                    pos = start + word.len();
                    
                    if i == 0 && instructions.contains(&word.to_lowercase().as_str()) {
                        // first word is instruction
                        highlights.push(serde_json::json!({
                            "start": start,
                            "end": pos,
                            "type": "instruction"
                        }));
                    } else if word.starts_with(';') {
                        // comment - highlight rest of line
                        highlights.push(serde_json::json!({
                            "start": start,
                            "end": line.len(),
                            "type": "comment"
                        }));
                        break;
                    } else if word.ends_with(':') {
                        // label
                        highlights.push(serde_json::json!({
                            "start": start,
                            "end": pos,
                            "type": "label"
                        }));
                    } else if word.parse::<i32>().is_ok() {
                        // number literal
                        highlights.push(serde_json::json!({
                            "start": start,
                            "end": pos,
                            "type": "number"
                        }));
                    }
                }
            }
            "shell" => {
                // basic shell syntax highlighting
                if line.trim().starts_with('#') {
                    // comment line
                    highlights.push(serde_json::json!({
                        "start": 0,
                        "end": line.len(),
                        "type": "comment"
                    }));
                } else {
                    let keywords = ["if", "then", "else", "fi", "for", "do", "done", 
                                   "while", "case", "esac", "function"];
                    let builtins = ["echo", "cd", "ls", "pwd", "export", "source", 
                                   "alias", "unalias"];
                    
                    let words: Vec<&str> = line.split_whitespace().collect();
                    let mut pos = 0;
                    
                    for word in words {
                        let start = line[pos..].find(word).unwrap_or(0) + pos;
                        pos = start + word.len();
                        
                        if keywords.contains(&word) {
                            highlights.push(serde_json::json!({
                                "start": start,
                                "end": pos,
                                "type": "keyword"
                            }));
                        } else if builtins.contains(&word) {
                            highlights.push(serde_json::json!({
                                "start": start,
                                "end": pos,
                                "type": "builtin"
                            }));
                        } else if word.starts_with('"') || word.starts_with('\'') {
                            highlights.push(serde_json::json!({
                                "start": start,
                                "end": pos,
                                "type": "string"
                            }));
                        }
                    }
                }
            }
            "markdown" => {
                // basic markdown syntax highlighting
                if line.starts_with('#') {
                    let level = line.chars().take_while(|&c| c == '#').count();
                    highlights.push(serde_json::json!({
                        "start": 0,
                        "end": level,
                        "type": "heading"
                    }));
                } else if line.starts_with("```") {
                    highlights.push(serde_json::json!({
                        "start": 0,
                        "end": line.len(),
                        "type": "code_fence"
                    }));
                } else if line.starts_with("- ") || line.starts_with("* ") {
                    highlights.push(serde_json::json!({
                        "start": 0,
                        "end": 2,
                        "type": "list_marker"
                    }));
                }
                
                // inline code blocks `like this`
                let mut chars = line.chars().enumerate();
                let mut in_code = false;
                let mut code_start = 0;
                
                while let Some((i, ch)) = chars.next() {
                    if ch == '`' {
                        if in_code {
                            highlights.push(serde_json::json!({
                                "start": code_start,
                                "end": i + 1,
                                "type": "inline_code"
                            }));
                            in_code = false;
                        } else {
                            code_start = i;
                            in_code = true;
                        }
                    }
                }
            }
            _ => {
                // no syntax highlighting for plain text
            }
        }
        
        highlights
    }
}

// create assembly program templates
// used by nano editor for quick scaffolding
#[wasm_bindgen]
pub fn get_assembly_template(template_type: &str) -> String {
    match template_type {
        "basic" => {
            "# Basic Assembly Program\n\
             # Use 'cpu run <filename>' to execute\n\
             \n\
             # Push values onto stack\n\
             push 10\n\
             push 20\n\
             \n\
             # Add them\n\
             add\n\
             \n\
             # Print result\n\
             print\n\
             \n\
             # Exit program\n\
             halt\n".to_string()
        }
        "hello" => {
            "# Hello World Program\n\
             # Prints \"Hello, World!\" to the output\n\
             \n\
             push 72  # H\n\
             printchar\n\
             push 101 # e\n\
             printchar\n\
             push 108 # l\n\
             printchar\n\
             push 108 # l\n\
             printchar\n\
             push 111 # o\n\
             printchar\n\
             push 44  # ,\n\
             printchar\n\
             push 32  # space\n\
             printchar\n\
             push 87  # W\n\
             printchar\n\
             push 111 # o\n\
             printchar\n\
             push 114 # r\n\
             printchar\n\
             push 108 # l\n\
             printchar\n\
             push 100 # d\n\
             printchar\n\
             push 33  # !\n\
             printchar\n\
             halt\n".to_string()
        }
        "loop" => {
            "# Loop Example\n\
             # Prints numbers from 0 to 9\n\
             \n\
             push 0   # Counter\n\
             \n\
             loop:\n\
               dup\n\
               print  # Print current counter\n\
               \n\
               push 1\n\
               add    # Increment counter\n\
               \n\
               dup    # Duplicate counter for comparison\n\
               push 10 # Compare with limit\n\
               cmp    # Compare counter with limit\n\
               push -1 # -1 means counter < limit\n\
               cmp    # Check if previous result was -1\n\
               jumpifz loop # Jump if counter < limit\n\
               \n\
             halt\n".to_string()
        }
        _ => get_assembly_template("basic") // fallback to basic template
    }
}
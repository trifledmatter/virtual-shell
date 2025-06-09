use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use serde_json::json;

pub struct EditCommand;

impl Command for EditCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            // show help if no filename provided
            return Ok(String::from(
                "edit - Simple line-based text editor\n\
                 Usage: edit <filename>\n\
                 \n\
                 Commands:\n\
                 :q  - Quit without saving\n\
                 :w  - Save file\n\
                 :wq - Save and quit\n\
                 \n\
                 Line editing:\n\
                 <line_number> <content> - Write content to specific line (preserves spaces)\n\
                 * <content>             - Apply content to ALL lines\n\
                 Examples:\n\
                 5 push 10      - Write 'push 10' to line 5\n\
                 15    halt     - Write '   halt' to line 15 (with spaces)\n\
                 1 mov  eax,  1 - Preserves all spacing in assembly\n\
                 * ;            - Comment out all lines with semicolon"
            ));
        }
        
        let filename = &args[0];
        // handle relative/absolute paths like everywhere else
        let path = if filename.starts_with('/') {
            filename.to_string()
        } else {
            format!("{}/{}", ctx.cwd, filename)
        };
        
        // try to read existing file, create empty if doesn't exist
        let content = match ctx.vfs.read_file(&path) {
            Ok(bytes) => String::from_utf8(bytes.to_vec()).unwrap_or_default(),
            Err(_) => String::new(), // new file, no biggie
        };
        
        // setup editor state in context vars
        ctx.set_var("_edit_file", &path);
        ctx.set_var("_edit_mode", "active");
        ctx.set_var("_edit_buffer", &content);
        ctx.set_var("_edit_modified", "false");
        
        // show the editor to user
        render_editor(ctx, &path, &content)
    }
}

// render editor state as json for frontend
fn render_editor(ctx: &TerminalContext, filename: &str, content: &str) -> CommandResult {
    let lines: Vec<&str> = if content.is_empty() {
        vec![]
    } else {
        content.lines().collect()
    };
    
    let modified = ctx.get_var("_edit_modified")
        .map(|s| s == "true")
        .unwrap_or(false);
    
    // build json structure for frontend display
    let editor_data = json!({
        "type": "edit_editor",
        "filename": filename,
        "modified": modified,
        "lines": lines.iter().enumerate().map(|(i, line)| {
            json!({
                "number": i + 1,
                "content": line
            })
        }).collect::<Vec<_>>(),
        "total_lines": lines.len(),
        "help": "Commands: :q (quit), :w (save), :wq (save & quit) | Line edit: <number> <content> or * <content> (preserves spaces)"
    });
    
    Ok(serde_json::to_string_pretty(&editor_data).unwrap())
}

// handles user input while in editor mode
pub struct EditInputCommand;

impl Command for EditInputCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // make sure we're actually in edit mode
        if ctx.get_var("_edit_mode").map(|s| s.as_str()) != Some("active") {
            return Err("Not in edit mode. Use 'edit <filename>' first.".to_string());
        }
        
        let filename = ctx.get_var("_edit_file")
            .ok_or("No file being edited")?
            .clone();
        
        if args.is_empty() {
            return Err("No input provided. Use :q, :w, :wq, or <line_number> <content>".to_string());
        }
        
        // entire input is single arg to preserve spaces - important for assembly
        let input = &args[0];
        
        // handle vim-style commands
        match input.as_str() {
            ":q" => {
                // quit without saving - clear editor state
                ctx.set_var("_edit_mode", "");
                ctx.set_var("_edit_file", "");
                ctx.set_var("_edit_buffer", "");
                ctx.set_var("_edit_modified", "");
                Ok("Exited editor without saving.".to_string())
            }
            ":w" => {
                // save file
                save_file(ctx, &filename)
            }
            ":wq" => {
                // save and quit - do both operations
                let save_result = save_file(ctx, &filename);
                ctx.set_var("_edit_mode", "");
                ctx.set_var("_edit_file", "");
                ctx.set_var("_edit_buffer", "");
                ctx.set_var("_edit_modified", "");
                save_result
            }
            _ => {
                // parse line editing commands: <line_number> <content> or * <content>
                let parts: Vec<&str> = input.splitn(2, ' ').collect();
                if parts.len() >= 1 {
                    if parts[0] == "*" {
                        // apply content to all lines - useful for commenting
                        let content = if parts.len() > 1 { parts[1] } else { "" };
                        edit_all_lines(ctx, &filename, content)
                    } else if let Ok(line_num) = parts[0].parse::<usize>() {
                        // edit specific line number
                        let content = if parts.len() > 1 { parts[1] } else { "" };
                        edit_line(ctx, &filename, line_num, content)
                    } else {
                        // if not a number or '*', append to next empty line or add as new line
                        let content = input;
                        let buffer = ctx.get_var("_edit_buffer")
                            .map(|s| s.clone())
                            .unwrap_or_else(|| String::new());
                        let mut lines: Vec<String> = if buffer.is_empty() {
                            vec![]
                        } else {
                            buffer.lines().map(|s| s.to_string()).collect()
                        };
                        // find first empty line
                        let mut added = false;
                        for line in lines.iter_mut() {
                            if line.trim().is_empty() {
                                *line = content.to_string();
                                added = true;
                                break;
                            }
                        }
                        if !added {
                            lines.push(content.to_string());
                        }
                        let new_buffer = lines.join("\n");
                        ctx.set_var("_edit_buffer", &new_buffer);
                        ctx.set_var("_edit_modified", "true");
                        render_editor(ctx, &filename, &new_buffer)
                    }
                } else {
                    Err("Invalid input format. Use <line_number> <content> or * <content>".to_string())
                }
            }
        }
    }
}

// save buffer to file
fn save_file(ctx: &mut TerminalContext, filename: &str) -> CommandResult {
    let buffer = ctx.get_var("_edit_buffer")
        .map(|s| s.clone())
        .unwrap_or_else(|| String::new());
    
    // try to write, create if doesn't exist
    let result = ctx.vfs.write_file(filename, buffer.as_bytes().to_vec())
        .or_else(|_| ctx.vfs.create_file(filename, buffer.as_bytes().to_vec()));
    
    match result {
        Ok(_) => {
            ctx.set_var("_edit_modified", "false");
            Ok(format!("Saved {} ({} bytes)", filename, buffer.len()))
        }
        Err(e) => Err(format!("Error saving {}: {}", filename, e)),
    }
}

// edit content of specific line number
fn edit_line(ctx: &mut TerminalContext, filename: &str, line_num: usize, content: &str) -> CommandResult {
    if line_num == 0 {
        return Err("Line numbers start from 1".to_string());
    }
    
    let buffer = ctx.get_var("_edit_buffer")
        .map(|s| s.clone())
        .unwrap_or_else(|| String::new());
    
    let mut lines: Vec<String> = if buffer.is_empty() {
        vec![]
    } else {
        buffer.lines().map(|s| s.to_string()).collect()
    };
    
    // extend file with empty lines if needed
    while lines.len() < line_num {
        lines.push(String::new());
    }
    
    // set the line content (convert from 1-based to 0-based indexing)
    lines[line_num - 1] = content.to_string();
    
    // update buffer and mark as modified
    let new_buffer = lines.join("\n");
    ctx.set_var("_edit_buffer", &new_buffer);
    ctx.set_var("_edit_modified", "true");
    
    // show updated editor view
    render_editor(ctx, filename, &new_buffer)
}

// apply same content to all existing lines
fn edit_all_lines(ctx: &mut TerminalContext, filename: &str, content: &str) -> CommandResult {
    let buffer = ctx.get_var("_edit_buffer")
        .map(|s| s.clone())
        .unwrap_or_else(|| String::new());
    
    let lines: Vec<String> = if buffer.is_empty() {
        // if file is empty, create one line with the content
        vec![content.to_string()]
    } else {
        // replace all existing lines with same content
        buffer.lines().map(|_| content.to_string()).collect()
    };
    
    // update buffer and mark as modified
    let new_buffer = lines.join("\n");
    ctx.set_var("_edit_buffer", &new_buffer);
    ctx.set_var("_edit_modified", "true");
    
    // show updated editor view
    render_editor(ctx, filename, &new_buffer)
}
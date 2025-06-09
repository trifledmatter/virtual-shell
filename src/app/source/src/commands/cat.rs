use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;

pub struct CatCommand;

impl Command for CatCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.is_empty() {
            return Ok(String::from(
                "cat - Display file contents\n\
                 Usage: cat [options] <file1> [file2] ...\n\
                 \n\
                 Options:\n\
                 -n, --number         Number all output lines\n\
                 -b, --number-nonblank Number non-empty output lines, overrides -n\n\
                 -s, --squeeze-blank   Suppress repeated empty output lines\n\
                 -h, --help           Display this help\n\
                 \n\
                 Examples:\n\
                 cat file.txt         Display contents of file.txt\n\
                 cat -n file.txt      Display with line numbers\n\
                 cat file1 file2      Display multiple files concatenated"
            ));
        }

        // options tracking
        let mut number_lines = false;
        let mut number_nonblank = false;
        let mut squeeze_blank = false;
        let mut files = Vec::new();

        // parse args - simple flag loop
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-n" | "--number" => {
                    number_lines = true;
                }
                "-b" | "--number-nonblank" => {
                    number_nonblank = true;
                    number_lines = false; // -b overrides -n
                }
                "-s" | "--squeeze-blank" => {
                    squeeze_blank = true;
                }
                "-h" | "--help" => {
                    return Ok(String::from(
                        "cat - Display file contents\n\
                         Usage: cat [options] <file1> [file2] ...\n\
                         \n\
                         Options:\n\
                         -n, --number         Number all output lines\n\
                         -b, --number-nonblank Number non-empty output lines, overrides -n\n\
                         -s, --squeeze-blank   Suppress repeated empty output lines\n\
                         -h, --help           Display this help"
                    ));
                }
                _ => {
                    // actual file or bad flag
                    if args[i].starts_with('-') {
                        return Err(format!("cat: invalid option '{}'", args[i]));
                    }
                    files.push(&args[i]);
                }
            }
            i += 1;
        }

        // bail early if no files
        if files.is_empty() {
            return Err("cat: missing file operand".to_string());
        }

        // output setup
        let mut output = String::new();
        let mut line_number = 1;
        let mut last_line_was_empty = false;

        // process each file
        for (file_index, filename) in files.iter().enumerate() {
            // convert relative to absolute path
            let path = if filename.starts_with('/') {
                filename.to_string()
            } else {
                format!("{}/{}", ctx.cwd, filename)
            };

            // try to read the file
            let content = match ctx.vfs.read_file(&path) {
                Ok(bytes) => {
                    // check if file is text or binary
                    match String::from_utf8(bytes.to_vec()) {
                        Ok(text) => text,
                        Err(_) => {
                            // binary file - just report and skip
                            output.push_str(&format!("cat: {}: Binary file (not displayed)\n", filename));
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // file not found - report and continue
                    output.push_str(&format!("cat: {}: No such file or directory\n", filename));
                    continue;
                }
            };

            // add spacing between files if needed
            if file_index > 0 && !output.is_empty() {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                // add blank line between files unless squeeze is on
                if !squeeze_blank {
                    output.push('\n');
                }
            }

            // process content line by line
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let is_empty = line.trim().is_empty();
                
                // skip empty lines if squeeze is on
                if squeeze_blank && is_empty && last_line_was_empty {
                    continue;
                }

                // handle line numbering based on flags
                if number_nonblank {
                    if !is_empty {
                        output.push_str(&format!("{:6}\t", line_number));
                        line_number += 1;
                    } else {
                        output.push_str("      \t");
                    }
                } else if number_lines {
                    output.push_str(&format!("{:6}\t", line_number));
                    line_number += 1;
                }

                output.push_str(line);
                
                // add newline unless it's the last line and doesn't have one
                if i < lines.len() - 1 || file_index < files.len() - 1 || content.ends_with('\n') {
                    output.push('\n');
                }

                last_line_was_empty = is_empty;
            }

            // special case for empty files
            if content.is_empty() {
                if number_lines {
                    output.push_str(&format!("{:6}\t\n", line_number));
                    line_number += 1;
                } else {
                    output.push('\n');
                }
                last_line_was_empty = true;
            }
        }

        // clean up trailing newline to match real cat
        if output.ends_with('\n') {
            output.pop();
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TerminalContext;
    use crate::vfs::VFS;

    #[test]
    fn test_cat_single_file() {
        let mut vfs = VFS::new();
        vfs.create_file("/test.txt", b"Hello\nWorld").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        let cmd = CatCommand;
        
        let result = cmd.execute(&["test.txt".to_string()], &mut ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello\nWorld");
    }

    #[test]
    fn test_cat_with_line_numbers() {
        let mut vfs = VFS::new();
        vfs.create_file("/test.txt", b"Line 1\nLine 2\n").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        let cmd = CatCommand;
        
        let result = cmd.execute(&["-n".to_string(), "test.txt".to_string()], &mut ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "     1\tLine 1\n     2\tLine 2\n     3\t");
    }

    #[test]
    fn test_cat_nonexistent_file() {
        let vfs = VFS::new();
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        let cmd = CatCommand;
        
        let result = cmd.execute(&["nonexistent.txt".to_string()], &mut ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No such file or directory"));
    }

    #[test]
    fn test_cat_multiple_files() {
        let mut vfs = VFS::new();
        vfs.create_file("/file1.txt", b"Content 1").unwrap();
        vfs.create_file("/file2.txt", b"Content 2").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        let cmd = CatCommand;
        
        let result = cmd.execute(&["file1.txt".to_string(), "file2.txt".to_string()], &mut ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Content 1\n\nContent 2");
    }

    #[test]
    fn test_cat_help() {
        let vfs = VFS::new();
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        let cmd = CatCommand;
        
        let result = cmd.execute(&["-h".to_string()], &mut ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Display file contents"));
    }
} 
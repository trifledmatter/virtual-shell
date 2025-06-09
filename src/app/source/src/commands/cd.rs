use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;

pub struct CdCommand;

impl Command for CdCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        let target_dir = if args.is_empty() {
            // cd with no args goes home, classic unix behavior
            "/home".to_string()
        } else if args.len() == 1 {
            args[0].clone()
        } else {
            return Err("cd: too many arguments".to_string());
        };

        // handle all the special cd shortcuts
        let (new_path, show_path) = match target_dir.as_str() {
            "-" => {
                // cd - swaps to previous directory
                match ctx.get_var("OLDPWD") {
                    Some(oldpwd) => (oldpwd.clone(), true),
                    None => {
                        return Err("cd: OLDPWD not set".to_string());
                    }
                }
            }
            "~" => {
                // cd ~ goes home
                ("/home".to_string(), false)
            }
            path if path.starts_with("~/") => {
                // cd ~/something expands tilde
                (format!("/home{}", &path[1..]), false)
            }
            "." => {
                // cd . stays put (why would you do this?)
                (ctx.cwd.clone(), false)
            }
            ".." => {
                // cd .. goes up one level
                (get_parent_directory(&ctx.cwd), false)
            }
            path if path.starts_with('/') => {
                // absolute path - use as is
                (path.to_string(), false)
            }
            path => {
                // relative path - resolve from current dir
                (resolve_relative_path(&ctx.cwd, path), false)
            }
        };

        // clean up path (remove redundant . and .. stuff)
        let normalized_path = normalize_path(&new_path);

        // check if target exists and is actually a directory
        match ctx.vfs.resolve_path(&normalized_path) {
            Some(node) => {
                match node {
                    VfsNode::Directory { .. } => {
                        // save current dir as OLDPWD for cd -
                        let old_cwd = ctx.cwd.clone();
                        ctx.set_var("OLDPWD", &old_cwd);
                        
                        // update current directory
                        ctx.cwd = normalized_path.clone();
                        
                        // update PWD env var too
                        ctx.set_var("PWD", &normalized_path);
                        
                        // show path if requested (cd - prints where it went)
                        if show_path {
                            Ok(normalized_path)
                        } else {
                            Ok(String::new()) // normal cd is silent
                        }
                    }
                    _ => {
                        Err(format!("cd: {}: Not a directory", target_dir))
                    }
                }
            }
            None => {
                Err(format!("cd: {}: No such file or directory", target_dir))
            }
        }
    }
}

// get parent directory path - handles edge cases like root
fn get_parent_directory(current_path: &str) -> String {
    if current_path == "/" {
        "/".to_string() // can't go above root
    } else {
        let parts: Vec<&str> = current_path.trim_end_matches('/').split('/').collect();
        if parts.len() <= 1 {
            "/".to_string()
        } else {
            let parent_parts = &parts[0..parts.len()-1];
            if parent_parts.is_empty() || parent_parts == [""] {
                "/".to_string()
            } else {
                parent_parts.join("/")
            }
        }
    }
}

// resolve relative path against current directory
fn resolve_relative_path(current_path: &str, relative_path: &str) -> String {
    if relative_path.is_empty() {
        return current_path.to_string();
    }
    
    // normalize current path (remove trailing slash)
    let base = if current_path.ends_with('/') {
        current_path.trim_end_matches('/').to_string()
    } else {
        current_path.to_string()
    };
    
    // join paths correctly
    if base == "/" {
        format!("/{}", relative_path)
    } else {
        format!("{}/{}", base, relative_path)
    }
}

// normalize path by resolving . and .. components
fn normalize_path(path: &str) -> String {
    let mut components = Vec::new();
    
    // split and process each path component
    for component in path.split('/') {
        match component {
            "" | "." => {
                // skip empty parts and current dir refs
                continue;
            }
            ".." => {
                // parent dir - pop last component if possible
                if !components.is_empty() && components.last() != Some(&"..".to_string()) {
                    components.pop();
                } else if !path.starts_with('/') {
                    // for relative paths, keep .. components
                    components.push("..".to_string());
                }
                // for absolute paths, .. at root is ignored
            }
            comp => {
                components.push(comp.to_string());
            }
        }
    }
    
    // reconstruct the normalized path
    if path.starts_with('/') {
        // absolute path
        if components.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", components.join("/"))
        }
    } else {
        // relative path
        if components.is_empty() {
            ".".to_string()
        } else {
            components.join("/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::TerminalContext;
    use crate::vfs::VFS;

    #[test]
    fn test_get_parent_directory() {
        assert_eq!(get_parent_directory("/"), "/");
        assert_eq!(get_parent_directory("/home"), "/");
        assert_eq!(get_parent_directory("/home/user"), "/home");
        assert_eq!(get_parent_directory("/home/user/docs"), "/home/user");
    }

    #[test]
    fn test_resolve_relative_path() {
        assert_eq!(resolve_relative_path("/home", "user"), "/home/user");
        assert_eq!(resolve_relative_path("/home/", "user"), "/home/user");
        assert_eq!(resolve_relative_path("/", "home"), "/home");
        assert_eq!(resolve_relative_path("/home/user", "docs"), "/home/user/docs");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/home/user/../docs"), "/home/docs");
        assert_eq!(normalize_path("/home/./user"), "/home/user");
        assert_eq!(normalize_path("/home//user"), "/home/user");
        assert_eq!(normalize_path("/home/user/.."), "/home");
        assert_eq!(normalize_path("/.."), "/");
        assert_eq!(normalize_path("user/../docs"), "docs");
        assert_eq!(normalize_path("./user"), "user");
        assert_eq!(normalize_path(".."), "..");
    }

    #[test]
    fn test_cd_absolute_path() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        vfs.create_directory("/home/user").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/".to_string();
        
        let cmd = CdCommand;
        let result = cmd.execute(&["/home".to_string()], &mut ctx);
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/home");
    }

    #[test]
    fn test_cd_relative_path() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        vfs.create_directory("/home/user").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/home".to_string();
        
        let cmd = CdCommand;
        let result = cmd.execute(&["user".to_string()], &mut ctx);
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/home/user");
    }

    #[test]
    fn test_cd_parent_directory() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        vfs.create_directory("/home/user").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/home/user".to_string();
        
        let cmd = CdCommand;
        let result = cmd.execute(&["..".to_string()], &mut ctx);
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/home");
    }

    #[test]
    fn test_cd_nonexistent_directory() {
        let vfs = VFS::new();
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        
        let cmd = CdCommand;
        let result = cmd.execute(&["nonexistent".to_string()], &mut ctx);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file or directory"));
    }

    #[test]
    fn test_cd_to_file() {
        let mut vfs = VFS::new();
        vfs.create_file("/test.txt", b"content").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        
        let cmd = CdCommand;
        let result = cmd.execute(&["test.txt".to_string()], &mut ctx);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not a directory"));
    }

    #[test]
    fn test_cd_home() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/some/path".to_string();
        
        let cmd = CdCommand;
        let result = cmd.execute(&[], &mut ctx); // cd with no args
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/home");
    }

    #[test]
    fn test_cd_tilde() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/some/path".to_string();
        
        let cmd = CdCommand;
        let result = cmd.execute(&["~".to_string()], &mut ctx);
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/home");
    }

    #[test]
    fn test_cd_previous_directory() {
        let mut vfs = VFS::new();
        vfs.create_directory("/home").unwrap();
        vfs.create_directory("/tmp").unwrap();
        
        let mut ctx = TerminalContext::new_with_vfs(vfs);
        ctx.cwd = "/home".to_string();
        ctx.set_var("OLDPWD", "/tmp");
        
        let cmd = CdCommand;
        let result = cmd.execute(&["-".to_string()], &mut ctx);
        
        assert!(result.is_ok());
        assert_eq!(ctx.cwd, "/tmp");
    }
} 
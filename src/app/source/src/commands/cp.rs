use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::{VfsNode, Permissions};
use chrono::Local;

pub struct CpCommand;

const CP_VERSION: &str = "cp 1.0.0";
const CP_HELP: &str = "Usage: cp [OPTION]... [-T] SOURCE DEST\n       cp [OPTION]... SOURCE... DIRECTORY\n       cp [OPTION]... -t DIRECTORY SOURCE...\nCopy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.\n\n  -R, -r, --recursive   copy directories recursively\n  -f, --force           if an existing destination file cannot be opened, remove it and try again\n  -i, --interactive     prompt before overwrite\n  -n, --no-clobber      do not overwrite an existing file\n  -v, --verbose         explain what is being done\n      --help            display this help and exit\n      --version         output version information and exit";

impl Command for CpCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        // handle help and version flags first
        if args.iter().any(|a| a == "--help") {
            return Ok(CP_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(CP_VERSION.to_string());
        }
        
        // parse all the flags cp supports
        let mut recursive = false;
        let mut force = false;
        let mut no_clobber = false;
        let mut verbose = false;
        let mut interactive = false;
        let mut sources = vec![];
        let mut dest: Option<String> = None;
        let mut t_mode = false; // -T flag
        let mut target_dir = None; // -t flag
        let mut skip_next = false;
        
        // go through args and parse flags vs files
        for (i, arg) in args.iter().enumerate() {
            if skip_next { skip_next = false; continue; }
            match arg.as_str() {
                "-r" | "-R" | "--recursive" => recursive = true,
                "-f" | "--force" => force = true,
                "-n" | "--no-clobber" => no_clobber = true,
                "-v" | "--verbose" => verbose = true,
                "-i" | "--interactive" => interactive = true,
                "-T" | "--no-target-directory" => t_mode = true,
                "-t" | "--target-directory" => {
                    // -t takes next arg as target dir
                    if let Some(dir) = args.get(i+1) {
                        target_dir = Some(dir.clone());
                        skip_next = true;
                    } else {
                        return Err("cp: option requires an argument -- 't'".to_string());
                    }
                }
                s if s.starts_with('-') => {
                    return Err(format!("cp: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => sources.push(arg.clone()),
            }
        }
        
        // handle different cp modes based on flags
        if let Some(dir) = target_dir {
            // -t mode: copy all sources to specified directory
            if sources.is_empty() {
                return Err("cp: missing file operand".to_string());
            }
            return cp_to_dir(ctx, &sources, &dir, recursive, force, no_clobber, verbose, interactive);
        }
        
        if sources.len() < 2 {
            return Err("cp: missing file operand".to_string());
        }
        
        // split sources into source files and destination
        let (srcs, dst) = sources.split_at(sources.len() - 1);
        
        if t_mode {
            // -T mode: exactly one source to one dest, no directory interpretation
            if srcs.len() != 1 {
                return Err("cp: with -T, the destination must be a single file".to_string());
            }
            return cp_file(ctx, &srcs[0], &dst[0], recursive, force, no_clobber, verbose, interactive);
        }
        
        // normal mode: single file copy or multiple files to directory
        if srcs.len() == 1 {
            cp_file(ctx, &srcs[0], &dst[0], recursive, force, no_clobber, verbose, interactive)
        } else {
            cp_to_dir(ctx, srcs, &dst[0], recursive, force, no_clobber, verbose, interactive)
        }
    }
}

// copy single file/dir/symlink to destination
fn cp_file(ctx: &mut TerminalContext, src: &str, dst: &str, recursive: bool, force: bool, no_clobber: bool, verbose: bool, _interactive: bool) -> CommandResult {
    // get source node info - need to clone data to avoid borrow checker drama
    let (src_is_file, src_is_dir, src_content, src_permissions, src_target) = {
        let src_node = ctx.vfs.resolve_path_with_symlinks(src, false)
            .ok_or(format!("cp: cannot stat '{}': No such file or directory", src))?;
        match src_node {
            VfsNode::File { content, permissions, .. } => 
                (true, false, Some(content.clone()), *permissions, None),
            VfsNode::Directory { permissions, .. } => 
                (false, true, None, *permissions, None),
            VfsNode::Symlink { target, permissions, .. } => 
                (false, false, None, *permissions, Some(target.clone())),
        }
    };
    
    // get destination parent directory
    let (parent_path, dst_name) = crate::vfs::VirtualFileSystem::split_path(dst)?;
    let parent = ctx.vfs.resolve_path_mut(parent_path)
        .and_then(|node| match node {
            VfsNode::Directory { children, .. } => Some(children),
            _ => None,
        })
        .ok_or("cp: cannot create file: parent directory does not exist")?;
    
    // handle destination conflicts
    if parent.contains_key(dst_name) {
        if no_clobber {
            return Ok(String::new()); // silently skip
        }
        if !force {
            return Err(format!("cp: cannot overwrite '{}': File exists", dst));
        }
        parent.remove(dst_name); // force overwrite
    }
    
    // copy based on source type
    if src_is_file {
        // regular file copy
        parent.insert(dst_name.to_string(), VfsNode::File {
            name: dst_name.to_string(),
            content: src_content.unwrap(),
            permissions: src_permissions,
            mtime: Local::now(),
        });
        if verbose {
            Ok(format!("'{}' -> '{}'", src, dst))
        } else {
            Ok(String::new())
        }
    } else if src_is_dir && recursive {
        // recursive directory copy - this gets complicated
        cp_dir_recursive(ctx, src, dst, force, no_clobber, verbose)
    } else if src_target.is_some() {
        // symlink copy
        parent.insert(dst_name.to_string(), VfsNode::Symlink {
            name: dst_name.to_string(),
            target: src_target.unwrap(),
            permissions: src_permissions,
            mtime: Local::now(),
        });
        if verbose {
            Ok(format!("'{}' -> '{}'", src, dst))
        } else {
            Ok(String::new())
        }
    } else {
        // trying to copy dir without -r flag
        Err("cp: omitting directory (use -r to copy directories)".to_string())
    }
}

// recursively copy directory and all its contents
fn cp_dir_recursive(ctx: &mut TerminalContext, src: &str, dst: &str, force: bool, no_clobber: bool, verbose: bool) -> CommandResult {
    // create destination directory structure first
    let (parent_path, dst_name) = crate::vfs::VirtualFileSystem::split_path(dst)?;
    
    // get source directory metadata and child list
    let (src_permissions, src_children) = {
        let src_node = ctx.vfs.resolve_path(src)
            .ok_or(format!("cp: cannot access '{}': No such file or directory", src))?;
        match src_node {
            VfsNode::Directory { permissions, children, .. } => {
                // collect child names to avoid borrowing issues
                let child_names: Vec<String> = children.keys().cloned().collect();
                (*permissions, child_names)
            }
            _ => return Err(format!("cp: '{}' is not a directory", src)),
        }
    };
    
    // create the destination directory
    let parent = ctx.vfs.resolve_path_mut(parent_path)
        .and_then(|node| match node {
            VfsNode::Directory { children, .. } => Some(children),
            _ => None,
        })
        .ok_or("cp: cannot create directory: parent does not exist")?;
    
    // handle existing destination
    if parent.contains_key(dst_name) {
        if no_clobber {
            return Ok(String::new());
        }
        if !force {
            return Err(format!("cp: cannot overwrite '{}': File exists", dst));
        }
        parent.remove(dst_name);
    }
    
    // create empty destination directory
    parent.insert(dst_name.to_string(), VfsNode::Directory {
        name: dst_name.to_string(),
        children: std::collections::HashMap::new(),
        permissions: src_permissions,
        mtime: Local::now(),
    });
    
    // recursively copy all children
    let mut results = Vec::new();
    for child_name in src_children {
        let child_src = format!("{}/{}", src.trim_end_matches('/'), child_name);
        let child_dst = format!("{}/{}", dst.trim_end_matches('/'), child_name);
        
        match cp_file(ctx, &child_src, &child_dst, true, force, no_clobber, verbose, false) {
            Ok(msg) => {
                if !msg.is_empty() {
                    results.push(msg);
                }
            }
            Err(e) => return Err(e),
        }
    }
    
    if verbose {
        results.insert(0, format!("'{}' -> '{}'", src, dst));
    }
    
    Ok(results.join("\n"))
}

// copy multiple sources to target directory
fn cp_to_dir(ctx: &mut TerminalContext, srcs: &[String], dir: &str, recursive: bool, force: bool, no_clobber: bool, verbose: bool, interactive: bool) -> CommandResult {
    // verify destination is actually a directory
    let dir_node = ctx.vfs.resolve_path_with_symlinks(dir, false).ok_or(format!("cp: target '{}' is not a directory", dir))?;
    if !matches!(dir_node, VfsNode::Directory { .. }) {
        return Err(format!("cp: target '{}' is not a directory", dir));
    }
    
    // copy each source file to destination directory
    let mut results = Vec::new();
    for src in srcs {
        // extract filename from source path
        let file_name = src.split('/').last().unwrap_or(src);
        let dst = format!("{}/{}", dir.trim_end_matches('/'), file_name);
        let res = cp_file(ctx, src, &dst, recursive, force, no_clobber, verbose, interactive)?;
        if !res.is_empty() {
            results.push(res);
        }
    }
    Ok(results.join("\n"))
}

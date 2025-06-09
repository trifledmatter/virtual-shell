use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::{VfsNode, Permissions};
use chrono::Local;

/// mv [OPTION]... SOURCE... DEST
/// Rename SOURCE to DEST, or move SOURCE(s) to DIRECTORY.
pub struct MvCommand;

const MV_VERSION: &str = "mv 1.0.0";
const MV_HELP: &str = "Usage: mv [OPTION]... [-T] SOURCE DEST\n       mv [OPTION]... SOURCE... DIRECTORY\n       mv [OPTION]... -t DIRECTORY SOURCE...\nRename SOURCE to DEST, or move SOURCE(s) to DIRECTORY.\n\n  -f, --force           do not prompt before overwriting\n  -i, --interactive     prompt before overwrite\n  -n, --no-clobber      do not overwrite an existing file\n  -v, --verbose         explain what is being done\n  -T, --no-target-directory\n  -t, --target-directory=DIRECTORY\n      --help            display this help and exit\n      --version         output version information and exit";

impl Command for MvCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(MV_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(MV_VERSION.to_string());
        }
        let mut force = false;
        let mut no_clobber = false;
        let mut verbose = false;
        let mut interactive = false;
        let mut sources = vec![];
        let mut t_mode = false;
        let mut target_dir = None;
        let mut skip_next = false;
        for (i, arg) in args.iter().enumerate() {
            if skip_next { skip_next = false; continue; }
            match arg.as_str() {
                "-f" | "--force" => force = true,
                "-n" | "--no-clobber" => no_clobber = true,
                "-v" | "--verbose" => verbose = true,
                "-i" | "--interactive" => interactive = true,
                "-T" | "--no-target-directory" => t_mode = true,
                "-t" | "--target-directory" => {
                    if let Some(dir) = args.get(i+1) {
                        target_dir = Some(dir.clone());
                        skip_next = true;
                    } else {
                        return Err("mv: option requires an argument -- 't'".to_string());
                    }
                }
                s if s.starts_with('-') => {
                    return Err(format!("mv: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => sources.push(arg.clone()),
            }
        }
        if let Some(dir) = target_dir {
            if sources.is_empty() {
                return Err("mv: missing file operand".to_string());
            }
            return mv_to_dir(ctx, &sources, &dir, force, no_clobber, verbose, interactive);
        }
        if sources.len() < 2 {
            return Err("mv: missing file operand".to_string());
        }
        let (srcs, dst) = sources.split_at(sources.len() - 1);
        if t_mode {
            if srcs.len() != 1 {
                return Err("mv: with -T, the destination must be a single file".to_string());
            }
            return mv_file(ctx, &srcs[0], &dst[0], force, no_clobber, verbose, interactive);
        }
        if srcs.len() == 1 {
            mv_file(ctx, &srcs[0], &dst[0], force, no_clobber, verbose, interactive)
        } else {
            mv_to_dir(ctx, srcs, &dst[0], force, no_clobber, verbose, interactive)
        }
    }
}

fn mv_file(ctx: &mut TerminalContext, src: &str, dst: &str, force: bool, no_clobber: bool, verbose: bool, _interactive: bool) -> CommandResult {
    // bail if source doesn't exist
    if ctx.vfs.resolve_path_with_symlinks(src, false).is_none() {
        return Err(format!("mv: cannot stat '{}': No such file or directory", src));
    }
    
    // get parent dirs and filenames for both src and dst
    let (src_parent_path, src_name) = crate::vfs::VirtualFileSystem::split_path(src)?;
    let (dst_parent_path, dst_name) = crate::vfs::VirtualFileSystem::split_path(dst)?;
    
    // moving within same dir is simpler - just rename
    if src_parent_path == dst_parent_path {
        let parent = ctx.vfs.resolve_path_mut(src_parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("mv: cannot move: parent directory does not exist")?;
        
        // nothing to do if src and dst are identical
        if dst_name == src_name {
            return Ok(String::new());
        }
        
        // handle destination already exists case
        if parent.contains_key(dst_name) {
            if no_clobber {
                return Ok(String::new()); // silently skip if no-clobber
            }
            if !force {
                return Err(format!("mv: cannot overwrite '{}': File exists", dst));
            }
            parent.remove(dst_name); // force overwrite
        }
        
        // do the actual move - remove from src and add to dst
        let node = parent.remove(src_name).ok_or("mv: source not found")?;
        parent.insert(dst_name.to_string(), node);
    } else {
        // cross-directory move - extract from src, then insert into dst
        
        // grab the node from source dir
        let node = {
            let src_parent = ctx.vfs.resolve_path_mut(src_parent_path)
                .and_then(|node| match node {
                    VfsNode::Directory { children, .. } => Some(children),
                    _ => None,
                })
                .ok_or("mv: cannot move: source parent directory does not exist")?;
            
            src_parent.remove(src_name).ok_or("mv: source not found")?
        };
        
        // get the destination dir
        let dst_parent = ctx.vfs.resolve_path_mut(dst_parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("mv: cannot move: destination parent directory does not exist")?;
        
        // handle if destination already exists
        if dst_parent.contains_key(dst_name) {
            if no_clobber {
                // put the node back in source since we're not moving
                let src_parent = ctx.vfs.resolve_path_mut(src_parent_path)
                    .and_then(|node| match node {
                        VfsNode::Directory { children, .. } => Some(children),
                        _ => None,
                    })
                    .unwrap();
                src_parent.insert(src_name.to_string(), node);
                return Ok(String::new());
            }
            if !force {
                // put the node back in source since we're erroring
                let src_parent = ctx.vfs.resolve_path_mut(src_parent_path)
                    .and_then(|node| match node {
                        VfsNode::Directory { children, .. } => Some(children),
                        _ => None,
                    })
                    .unwrap();
                src_parent.insert(src_name.to_string(), node);
                return Err(format!("mv: cannot overwrite '{}': File exists", dst));
            }
            dst_parent.remove(dst_name); // force overwrite
        }
        
        // finally insert the node at destination
        dst_parent.insert(dst_name.to_string(), node);
    }
    
    // only print output in verbose mode
    if verbose {
        Ok(format!("'{}' -> '{}'", src, dst))
    } else {
        Ok(String::new())
    }
}

fn mv_to_dir(ctx: &mut TerminalContext, srcs: &[String], dir: &str, force: bool, no_clobber: bool, verbose: bool, interactive: bool) -> CommandResult {
    // make sure target dir exists and is actually a dir
    let dir_node = ctx.vfs.resolve_path_with_symlinks(dir, false).ok_or(format!("mv: target '{}' is not a directory", dir))?;
    if !matches!(dir_node, VfsNode::Directory { .. }) {
        return Err(format!("mv: target '{}' is not a directory", dir));
    }
    
    // move each source into the target dir
    let mut results = Vec::new();
    for src in srcs {
        // extract filename from path
        let file_name = src.split('/').last().unwrap_or(src);
        // build destination path
        let dst = format!("{}/{}", dir.trim_end_matches('/'), file_name);
        // do the move
        let res = mv_file(ctx, src, &dst, force, no_clobber, verbose, interactive)?;
        if !res.is_empty() {
            results.push(res);
        }
    }
    Ok(results.join("\n"))
}

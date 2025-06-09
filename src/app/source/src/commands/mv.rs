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
    // Get the source node info to determine what we're moving
    let (is_file, is_dir, file_content, is_symlink, symlink_target) = {
        let src_node = ctx.vfs.resolve_path_with_symlinks(src, false)
            .ok_or(format!("mv: cannot stat '{}': No such file or directory", src))?;
        match src_node {
            VfsNode::File { content, .. } => (true, false, Some(content.clone()), false, None),
            VfsNode::Directory { .. } => (false, true, None, false, None),
            VfsNode::Symlink { target, .. } => (false, false, None, true, Some(target.clone())),
        }
    };
    
    // Check if destination already exists
    if ctx.vfs.resolve_path(dst).is_some() {
        if no_clobber {
            return Ok(String::new()); // silently skip if no-clobber
        }
        if !force {
            return Err(format!("mv: cannot overwrite '{}': File exists", dst));
        }
        // For force overwrite, delete the destination first
        ctx.delete_with_events(dst)?;
    }
    
    // Create the destination based on source type
    if is_file {
        ctx.create_file_with_events(dst, &file_content.unwrap())?;
    } else if is_dir {
        // For directories, we need to recursively move the entire tree
        return mv_dir_recursive(ctx, src, dst, force, no_clobber, verbose);
    } else if is_symlink {
        ctx.create_symlink_with_events(dst, &symlink_target.unwrap())?;
    }
    
    // Delete the source after successful creation
    ctx.delete_with_events(src)?;
    
    // only print output in verbose mode
    if verbose {
        Ok(format!("'{}' -> '{}'", src, dst))
    } else {
        Ok(String::new())
    }
}

// Helper function to recursively move directories
fn mv_dir_recursive(ctx: &mut TerminalContext, src: &str, dst: &str, force: bool, no_clobber: bool, verbose: bool) -> CommandResult {
    // Get all children of the source directory
    let src_children = {
        let src_node = ctx.vfs.resolve_path(src).ok_or(format!("mv: cannot access '{}': No such file or directory", src))?;
        match src_node {
            VfsNode::Directory { children, .. } => {
                let child_names: Vec<String> = children.keys().cloned().collect();
                child_names
            }
            _ => return Err(format!("mv: '{}' is not a directory", src)),
        }
    };
    
    // Create the destination directory
    ctx.create_dir_with_events(dst)?;
    
    // Recursively move all children
    for child_name in src_children {
        let child_src = format!("{}/{}", src.trim_end_matches('/'), child_name);
        let child_dst = format!("{}/{}", dst.trim_end_matches('/'), child_name);
        
        mv_file(ctx, &child_src, &child_dst, force, no_clobber, false, false)?;
    }
    
    // Delete the empty source directory
    ctx.delete_with_events(src)?;
    
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

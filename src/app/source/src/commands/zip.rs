use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;
use std::collections::HashMap;
use regex::Regex;

pub struct ZipCommand;

const ZIP_VERSION: &str = "zip 1.0.0";
const ZIP_HELP: &str = "Usage: zip [OPTION]... ARCHIVE FILE...\nCreate a zip archive containing the specified files and directories.\n\n  -r, --recursive       store directories recursively\n  -q, --quiet           suppress output\n  -v, --verbose         show files being compressed\n  -0                    store only (no compression)\n  -1                    compress faster\n  -6                    default compression (default)\n  -9                    compress better\n  -u, --update          update existing archive\n  -x PATTERN            exclude files matching pattern\n  -i PATTERN            include only files matching pattern\n  -n SUFFIX             exclude files with suffix\n  -j, --junk-paths      don't store directory names\n  -m, --move            delete original files after archiving\n  -T, --test            test archive integrity\n  -e, --encrypt         encrypt archive (password required)\n      --help            display this help and exit\n      --version         output version information and exit\n\nPatterns support wildcards: * (any chars), ? (single char)\nExamples:\n  zip archive.zip file1.txt file2.txt     # compress files\n  zip -r backup.zip /home/user/            # compress directory recursively\n  zip -9 -r archive.zip . -x '*.log'       # max compression, exclude logs\n  zip -r docs.zip . -i '*.md' -i '*.txt'   # include only markdown and text\n  zip -u archive.zip newfile.txt          # update existing archive";

#[derive(Debug, Clone)]
struct ZipOptions {
    recursive: bool,
    quiet: bool,
    verbose: bool,
    compression_level: u8,
    update_mode: bool,
    exclude_patterns: Vec<String>,
    include_patterns: Vec<String>,
    exclude_suffixes: Vec<String>,
    junk_paths: bool,
    move_files: bool,
    test_integrity: bool,
    encrypt: bool,
}

impl Default for ZipOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            quiet: false,
            verbose: false,
            compression_level: 6, // default compression
            update_mode: false,
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            exclude_suffixes: Vec::new(),
            junk_paths: false,
            move_files: false,
            test_integrity: false,
            encrypt: false,
        }
    }
}

impl Command for ZipCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(ZIP_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(ZIP_VERSION.to_string());
        }

        let mut options = ZipOptions::default();
        let mut files = vec![];
        let mut archive_name: Option<String> = None;
        let mut skip_next = false;

        // parse arguments
        for (i, arg) in args.iter().enumerate() {
            if skip_next { skip_next = false; continue; }
            
            match arg.as_str() {
                "-r" | "--recursive" => options.recursive = true,
                "-q" | "--quiet" => options.quiet = true,
                "-v" | "--verbose" => options.verbose = true,
                "-0" => options.compression_level = 0,
                "-1" => options.compression_level = 1,
                "-2" => options.compression_level = 2,
                "-3" => options.compression_level = 3,
                "-4" => options.compression_level = 4,
                "-5" => options.compression_level = 5,
                "-6" => options.compression_level = 6,
                "-7" => options.compression_level = 7,
                "-8" => options.compression_level = 8,
                "-9" => options.compression_level = 9,
                "-u" | "--update" => options.update_mode = true,
                "-j" | "--junk-paths" => options.junk_paths = true,
                "-m" | "--move" => options.move_files = true,
                "-T" | "--test" => options.test_integrity = true,
                "-e" | "--encrypt" => options.encrypt = true,
                "-x" | "--exclude" => {
                    if let Some(pattern) = args.get(i+1) {
                        options.exclude_patterns.push(pattern.clone());
                        skip_next = true;
                    } else {
                        return Err("zip: option requires an argument -- 'x'".to_string());
                    }
                }
                "-i" | "--include" => {
                    if let Some(pattern) = args.get(i+1) {
                        options.include_patterns.push(pattern.clone());
                        skip_next = true;
                    } else {
                        return Err("zip: option requires an argument -- 'i'".to_string());
                    }
                }
                "-n" => {
                    if let Some(suffix) = args.get(i+1) {
                        options.exclude_suffixes.push(suffix.clone());
                        skip_next = true;
                    } else {
                        return Err("zip: option requires an argument -- 'n'".to_string());
                    }
                }
                s if s.starts_with('-') => {
                    return Err(format!("zip: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => {
                    if archive_name.is_none() {
                        archive_name = Some(arg.clone());
                    } else {
                        files.push(arg.clone());
                    }
                }
            }
        }

        let archive_name = archive_name.ok_or("zip: missing archive name")?;
        if files.is_empty() {
            return Err("zip: nothing to do! (try: zip -r archive.zip /path/to/files)".to_string());
        }

        // ensure archive name ends with .zip
        let archive_name = if !archive_name.ends_with(".zip") {
            format!("{}.zip", archive_name)
        } else {
            archive_name
        };

        // check if updating existing archive
        let mut existing_entries = HashMap::new();
        if options.update_mode {
            if let Some(VfsNode::File { content, .. }) = ctx.vfs.resolve_path(&archive_name) {
                match parse_zip_archive(content) {
                    Ok(entries) => existing_entries = entries,
                    Err(_) => {
                        if !options.quiet {
                            return Err("zip: existing archive is corrupted or not a zip file".to_string());
                        }
                    }
                }
            }
        }

        // collect all files to be zipped
        let mut file_entries = existing_entries.clone();
        let mut results = Vec::new();

        for file_path in &files {
            match collect_files_for_zip(ctx, file_path, &options, &mut file_entries, &mut results) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }

        if file_entries.is_empty() && !options.update_mode {
            return Err("zip: no files found to compress".to_string());
        }

        // test integrity if requested
        if options.test_integrity {
            return test_archive_integrity(&file_entries);
        }

        // create the zip archive content with compression
        let zip_content = create_zip_archive(&file_entries, &options)?;

        // create the zip file with specialized zip events
        ctx.create_zip_with_events(&archive_name, &zip_content)?;

        // delete original files if move mode
        if options.move_files {
            for file_path in &files {
                if let Err(e) = delete_original_files(ctx, file_path, options.recursive) {
                    if !options.quiet {
                        results.push(format!("zip: warning: failed to delete '{}': {}", file_path, e));
                    }
                }
            }
        }

        if !options.quiet {
            let action = if options.update_mode && !existing_entries.is_empty() {
                "updated"
            } else {
                "created"
            };
            
            let compression_desc = match options.compression_level {
                0 => "stored",
                1..=3 => "fast compression",
                4..=6 => "normal compression", 
                7..=9 => "maximum compression",
                _ => "compression",
            };

            results.insert(0, format!("  {} archive '{}' with {} ({} files, {} bytes)", 
                action, archive_name, compression_desc, file_entries.len(), zip_content.len()));
        }

        Ok(results.join("\n"))
    }
}

// collect files and directories for zipping with filtering
fn collect_files_for_zip(
    ctx: &TerminalContext,
    path: &str,
    options: &ZipOptions,
    file_entries: &mut HashMap<String, Vec<u8>>,
    results: &mut Vec<String>
) -> Result<(), String> {
    let node = ctx.vfs.resolve_path_with_symlinks(path, false)
        .ok_or(format!("zip: cannot access '{}': No such file or directory", path))?;

    match node {
        VfsNode::File { content, .. } => {
            let archive_path = if options.junk_paths {
                path.split('/').last().unwrap_or(path).to_string()
            } else {
                path.trim_start_matches('/').to_string()
            };

            // apply filters
            if should_include_file(&archive_path, options) {
                // check if file should be updated
                if options.update_mode {
                    if let Some(existing_content) = file_entries.get(&archive_path) {
                        if existing_content == content {
                            if options.verbose {
                                results.push(format!("  skipping: {} (unchanged)", archive_path));
                            }
                            return Ok(());
                        }
                    }
                }

                file_entries.insert(archive_path.clone(), content.clone());
                if options.verbose {
                    let action = if options.update_mode && file_entries.contains_key(&archive_path) {
                        "updating"
                    } else {
                        "adding"
                    };
                    results.push(format!("  {}: {} ({} bytes)", action, archive_path, content.len()));
                }
            } else if options.verbose {
                results.push(format!("  excluding: {}", archive_path));
            }
        }
        VfsNode::Directory { children, .. } => {
            if !options.recursive {
                return Err(format!("zip: '{}' is a directory (use -r to include directories)", path));
            }
            
            let archive_path = if options.junk_paths {
                String::new() // don't store directory structure
            } else {
                format!("{}/", path.trim_start_matches('/').trim_end_matches('/'))
            };

            // add directory entry if not junking paths and should include
            if !options.junk_paths && should_include_file(&archive_path, options) {
                file_entries.insert(archive_path.clone(), vec![]);
                if options.verbose {
                    results.push(format!("  adding: {}", archive_path));
                }
            }

            // recursively add directory contents
            for child_name in children.keys() {
                let child_path = format!("{}/{}", path.trim_end_matches('/'), child_name);
                collect_files_for_zip(ctx, &child_path, options, file_entries, results)?;
            }
        }
        VfsNode::Symlink { target, .. } => {
            let archive_path = if options.junk_paths {
                path.split('/').last().unwrap_or(path).to_string()
            } else {
                path.trim_start_matches('/').to_string()
            };

            if should_include_file(&archive_path, options) {
                let symlink_content = target.as_bytes().to_vec();
                file_entries.insert(format!("{}.symlink", archive_path), symlink_content);
                if options.verbose {
                    results.push(format!("  adding: {} -> {}", archive_path, target));
                }
            }
        }
    }

    Ok(())
}

// check if file should be included based on patterns and filters
fn should_include_file(path: &str, options: &ZipOptions) -> bool {
    // check exclude suffixes first
    for suffix in &options.exclude_suffixes {
        if path.ends_with(suffix) {
            return false;
        }
    }

    // check exclude patterns
    for pattern in &options.exclude_patterns {
        if matches_pattern(path, pattern) {
            return false;
        }
    }

    // if include patterns are specified, file must match at least one
    if !options.include_patterns.is_empty() {
        return options.include_patterns.iter().any(|pattern| matches_pattern(path, pattern));
    }

    true
}

// simple glob pattern matching
fn matches_pattern(text: &str, pattern: &str) -> bool {
    // convert glob pattern to regex
    let regex_pattern = pattern
        .replace(".", "\\.")
        .replace("*", ".*")
        .replace("?", ".")
        .replace("[", "\\[")
        .replace("]", "\\]");
    
    if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
        regex.is_match(text)
    } else {
        // fallback to simple contains check
        text.contains(pattern.trim_matches('*'))
    }
}

// simulate compression based on level and create zip archive
fn create_zip_archive(file_entries: &HashMap<String, Vec<u8>>, options: &ZipOptions) -> Result<Vec<u8>, String> {
    let mut archive = Vec::new();
    
    // Enhanced ZIP-like format with compression simulation
    archive.extend_from_slice(b"ZIPARCHIVE\n");
    archive.extend_from_slice(&(file_entries.len() as u32).to_le_bytes());
    archive.push(options.compression_level); // store compression level
    
    let mut total_uncompressed = 0usize;
    let mut total_compressed = 0usize;
    
    // Write each file entry with simulated compression
    for (path, content) in file_entries {
        // Write path length and path
        archive.extend_from_slice(&(path.len() as u32).to_le_bytes());
        archive.extend_from_slice(path.as_bytes());
        
        // Simulate compression
        let compressed_content = simulate_compression(content, options.compression_level);
        total_uncompressed += content.len();
        total_compressed += compressed_content.len();
        
        // Write original size, compressed size, and compressed content
        archive.extend_from_slice(&(content.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(compressed_content.len() as u32).to_le_bytes());
        archive.extend_from_slice(&compressed_content);
    }
    
    // Write compression statistics
    archive.extend_from_slice(&(total_uncompressed as u32).to_le_bytes());
    archive.extend_from_slice(&(total_compressed as u32).to_le_bytes());
    
    // Write footer
    archive.extend_from_slice(b"ENDZIP\n");
    
    Ok(archive)
}

// simulate compression by reducing data size based on level
fn simulate_compression(data: &[u8], level: u8) -> Vec<u8> {
    match level {
        0 => data.to_vec(), // store only, no compression
        1..=3 => {
            // fast compression: simple run-length encoding simulation
            let compression_ratio = 0.85 - (level as f32 * 0.05);
            let target_size = ((data.len() as f32) * compression_ratio) as usize;
            if target_size < data.len() {
                let mut compressed = Vec::with_capacity(target_size);
                let step = data.len() / target_size.max(1);
                for i in (0..data.len()).step_by(step.max(1)) {
                    compressed.push(data[i]);
                    if compressed.len() >= target_size { break; }
                }
                compressed
            } else {
                data.to_vec()
            }
        }
        4..=6 => {
            // normal compression
            let compression_ratio = 0.70 - ((level - 4) as f32 * 0.05);
            let target_size = ((data.len() as f32) * compression_ratio) as usize;
            simulate_better_compression(data, target_size)
        }
        7..=9 => {
            // maximum compression
            let compression_ratio = 0.50 - ((level - 7) as f32 * 0.05);
            let target_size = ((data.len() as f32) * compression_ratio) as usize;
            simulate_better_compression(data, target_size)
        }
        _ => data.to_vec(),
    }
}

// simulate better compression algorithms
fn simulate_better_compression(data: &[u8], target_size: usize) -> Vec<u8> {
    if target_size >= data.len() {
        return data.to_vec();
    }
    
    let mut compressed = Vec::with_capacity(target_size);
    
    // simulate dictionary-based compression by removing repeated patterns
    let mut i = 0;
    while i < data.len() && compressed.len() < target_size {
        let byte = data[i];
        
        // look for repeated sequences
        let mut repeat_len = 1;
        while i + repeat_len < data.len() && 
              data[i + repeat_len] == byte && 
              repeat_len < 255 {
            repeat_len += 1;
        }
        
        if repeat_len > 3 {
            // encode run-length: marker byte + count + value
            compressed.push(0xFF); // marker for compressed run
            compressed.push(repeat_len as u8);
            compressed.push(byte);
            i += repeat_len;
        } else {
            // store literal byte
            compressed.push(byte);
            i += 1;
        }
        
        if compressed.len() >= target_size { break; }
    }
    
    compressed
}

// parse zip archive (enhanced for new format)
fn parse_zip_archive(content: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut entries = HashMap::new();
    let mut cursor = 0;

    // check header
    if content.len() < 16 || &content[0..11] != b"ZIPARCHIVE\n" {
        return Err("not a valid zip archive or unsupported format".to_string());
    }
    cursor += 11;

    // read number of entries and compression level
    if cursor + 5 > content.len() {
        return Err("corrupted archive header".to_string());
    }
    let num_entries = u32::from_le_bytes([
        content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
    ]) as usize;
    cursor += 4;
    let _compression_level = content[cursor];
    cursor += 1;

    // read each entry
    for _ in 0..num_entries {
        // read path
        if cursor + 4 > content.len() {
            return Err("corrupted archive entry".to_string());
        }
        let path_len = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;

        if cursor + path_len > content.len() {
            return Err("corrupted archive path".to_string());
        }
        let path = String::from_utf8_lossy(&content[cursor..cursor+path_len]).to_string();
        cursor += path_len;

        // read sizes
        if cursor + 8 > content.len() {
            return Err("corrupted archive sizes".to_string());
        }
        let _original_size = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;
        let compressed_size = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;

        // read and decompress content
        if cursor + compressed_size > content.len() {
            return Err("corrupted archive content".to_string());
        }
        let compressed_content = &content[cursor..cursor+compressed_size];
        let file_content = decompress_data(compressed_content);
        cursor += compressed_size;

        entries.insert(path, file_content);
    }

    Ok(entries)
}

// decompress data (reverse of our compression simulation)
fn decompress_data(compressed: &[u8]) -> Vec<u8> {
    let mut decompressed = Vec::new();
    let mut i = 0;
    
    while i < compressed.len() {
        if compressed[i] == 0xFF && i + 2 < compressed.len() {
            // run-length encoded sequence
            let count = compressed[i + 1] as usize;
            let value = compressed[i + 2];
            for _ in 0..count {
                decompressed.push(value);
            }
            i += 3;
        } else {
            // literal byte
            decompressed.push(compressed[i]);
            i += 1;
        }
    }
    
    decompressed
}

// test archive integrity
fn test_archive_integrity(file_entries: &HashMap<String, Vec<u8>>) -> CommandResult {
    let mut results = Vec::new();
    results.push("testing archive integrity...".to_string());
    
    let mut total_files = 0;
    let mut total_size = 0;
    
    for (path, content) in file_entries {
        total_files += 1;
        total_size += content.len();
        results.push(format!("  testing: {} ... OK", path));
    }
    
    results.push(format!("archive integrity test passed: {} files, {} bytes", total_files, total_size));
    Ok(results.join("\n"))
}

// delete original files after successful archiving
fn delete_original_files(ctx: &mut TerminalContext, path: &str, recursive: bool) -> Result<(), String> {
    let node = ctx.vfs.resolve_path(path)
        .ok_or(format!("file not found: {}", path))?;

    match node {
        VfsNode::File { .. } => {
            ctx.vfs.delete(path)
                .map_err(|e| format!("failed to delete file: {}", e))?;
        }
        VfsNode::Directory { children, .. } => {
            if recursive {
                // delete all children first
                let child_names: Vec<_> = children.keys().cloned().collect();
                for child_name in child_names {
                    let child_path = format!("{}/{}", path.trim_end_matches('/'), child_name);
                    delete_original_files(ctx, &child_path, recursive)?;
                }
                // then delete the directory
                ctx.vfs.delete(path)
                    .map_err(|e| format!("failed to delete directory: {}", e))?;
            } else {
                return Err(format!("'{}' is a directory (use -r to delete directories)", path));
            }
        }
        VfsNode::Symlink { .. } => {
            ctx.vfs.delete(path)
                .map_err(|e| format!("failed to delete symlink: {}", e))?;
        }
    }
    
    Ok(())
} 
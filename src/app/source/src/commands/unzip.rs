use crate::command::{Command, CommandResult};
use crate::context::TerminalContext;
use crate::vfs::VfsNode;
use std::collections::HashMap;
use regex::Regex;

pub struct UnzipCommand;

const UNZIP_VERSION: &str = "unzip 1.0.0";
const UNZIP_HELP: &str = "Usage: unzip [OPTION]... ARCHIVE [FILE...] [DESTINATION]\nExtract files from a zip archive.\n\n  -d DIR            extract files into DIR\n  -l                list archive contents without extracting\n  -t                test archive integrity\n  -o                overwrite files without prompting\n  -n                never overwrite existing files\n  -f                freshen existing files only\n  -u                update files (extract if newer)\n  -j                junk paths (don't create directories)\n  -C                match filenames case-insensitively\n  -q, --quiet       suppress output\n  -v, --verbose     show files being extracted\n  -x PATTERN        exclude files matching pattern\n  -i PATTERN        include only files matching pattern\n  -P PASSWORD       use password for encrypted archives\n      --help        display this help and exit\n      --version     output version information and exit\n\nPatterns support wildcards: * (any chars), ? (single char)\nExamples:\n  unzip archive.zip                    # extract to current directory\n  unzip archive.zip -d /tmp/           # extract to /tmp/\n  unzip -l archive.zip                 # list contents only\n  unzip archive.zip '*.txt'            # extract only text files\n  unzip -x '*.log' archive.zip         # extract all except log files\n  unzip -t archive.zip                 # test integrity without extracting";

#[derive(Debug, Clone)]
struct UnzipOptions {
    list_only: bool,
    test_only: bool,
    overwrite: bool,
    never_overwrite: bool,
    freshen: bool,
    update: bool,
    junk_paths: bool,
    case_insensitive: bool,
    quiet: bool,
    verbose: bool,
    destination: Option<String>,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    file_patterns: Vec<String>,
    password: Option<String>,
}

impl Default for UnzipOptions {
    fn default() -> Self {
        Self {
            list_only: false,
            test_only: false,
            overwrite: false,
            never_overwrite: false,
            freshen: false,
            update: false,
            junk_paths: false,
            case_insensitive: false,
            quiet: false,
            verbose: false,
            destination: None,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            file_patterns: Vec::new(),
            password: None,
        }
    }
}

impl Command for UnzipCommand {
    fn execute(&self, args: &[String], ctx: &mut TerminalContext) -> CommandResult {
        if args.iter().any(|a| a == "--help") {
            return Ok(UNZIP_HELP.to_string());
        }
        if args.iter().any(|a| a == "--version") {
            return Ok(UNZIP_VERSION.to_string());
        }

        let mut options = UnzipOptions::default();
        let mut archive_name: Option<String> = None;
        let mut skip_next = false;

        // parse arguments
        for (i, arg) in args.iter().enumerate() {
            if skip_next { skip_next = false; continue; }
            match arg.as_str() {
                "-l" | "--list" => options.list_only = true,
                "-t" | "--test" => options.test_only = true,
                "-o" | "--overwrite" => options.overwrite = true,
                "-n" | "--never-overwrite" => options.never_overwrite = true,
                "-f" | "--freshen" => options.freshen = true,
                "-u" | "--update" => options.update = true,
                "-j" | "--junk-paths" => options.junk_paths = true,
                "-C" | "--case-insensitive" => options.case_insensitive = true,
                "-q" | "--quiet" => options.quiet = true,
                "-v" | "--verbose" => options.verbose = true,
                "-d" | "--directory" => {
                    if let Some(dir) = args.get(i+1) {
                        options.destination = Some(dir.clone());
                        skip_next = true;
                    } else {
                        return Err("unzip: option requires an argument -- 'd'".to_string());
                    }
                }
                "-x" | "--exclude" => {
                    if let Some(pattern) = args.get(i+1) {
                        options.exclude_patterns.push(pattern.clone());
                        skip_next = true;
                    } else {
                        return Err("unzip: option requires an argument -- 'x'".to_string());
                    }
                }
                "-i" | "--include" => {
                    if let Some(pattern) = args.get(i+1) {
                        options.include_patterns.push(pattern.clone());
                        skip_next = true;
                    } else {
                        return Err("unzip: option requires an argument -- 'i'".to_string());
                    }
                }
                "-P" | "--password" => {
                    if let Some(password) = args.get(i+1) {
                        options.password = Some(password.clone());
                        skip_next = true;
                    } else {
                        return Err("unzip: option requires an argument -- 'P'".to_string());
                    }
                }
                s if s.starts_with('-') => {
                    return Err(format!("unzip: unrecognized option '{}'. Try --help for more info.", s));
                }
                _ => {
                    if archive_name.is_none() {
                        archive_name = Some(arg.clone());
                    } else if options.destination.is_none() && !arg.contains('*') && !arg.contains('?') {
                        // could be destination directory
                        options.destination = Some(arg.clone());
                    } else {
                        // file pattern
                        options.file_patterns.push(arg.clone());
                    }
                }
            }
        }

        let archive_name = archive_name.ok_or("unzip: missing archive name")?;
        
        // Default destination should be a directory named after the zip file
        let default_destination = if let Some(stem) = archive_name.strip_suffix(".zip") {
            stem.to_string()
        } else {
            format!("{}_extracted", archive_name)
        };
        let destination = options.destination.clone().unwrap_or(default_destination);

        // read the zip archive
        let archive_content = match ctx.vfs.resolve_path(&archive_name) {
            Some(VfsNode::File { content, .. }) => content.clone(),
            Some(_) => return Err(format!("unzip: '{}' is not a file", archive_name)),
            None => return Err(format!("unzip: cannot find archive '{}'", archive_name)),
        };

        // parse the zip archive
        let file_entries = parse_zip_archive(&archive_content)?;

        if options.list_only {
            return list_archive_contents(&file_entries, &archive_name, &options);
        }

        if options.test_only {
            return test_archive_integrity(&file_entries, &archive_name);
        }

        // extract files
        extract_files(ctx, &file_entries, &archive_name, &destination, &options)
    }
}

// extract files with advanced filtering and options
fn extract_files(
    ctx: &mut TerminalContext,
    file_entries: &HashMap<String, (Vec<u8>, usize, usize)>, // (content, original_size, compressed_size)
    archive_name: &str,
    destination: &str,
    options: &UnzipOptions
) -> CommandResult {
    let mut results = Vec::new();
    let mut extracted_count = 0;
    let mut skipped_count = 0;
    let mut updated_count = 0;

    if !options.quiet {
        results.push(format!("Archive: {}", archive_name));
    }

    // Create the main extraction directory if it doesn't exist
    if ctx.vfs.resolve_path(destination).is_none() {
        ctx.create_dir_with_events(destination)?;
    }

    // filter files based on patterns
    let filtered_entries: Vec<_> = file_entries.iter()
        .filter(|(path, _)| should_extract_file(path, options))
        .collect();

    for (path, (content, original_size, compressed_size)) in filtered_entries {
        let extract_path = if options.junk_paths {
            // extract to destination with just filename
            let filename = path.split('/').last().unwrap_or(path);
            if destination == "/" {
                format!("/{}", filename)
            } else {
                format!("{}/{}", destination.trim_end_matches('/'), filename)
            }
        } else {
            // preserve directory structure
            if destination == "/" {
                format!("/{}", path.trim_start_matches('/'))
            } else {
                format!("{}/{}", destination.trim_end_matches('/'), path.trim_start_matches('/'))
            }
        };

        // check for existing files and handle according to options
        let file_exists = ctx.vfs.resolve_path(&extract_path).is_some();
        if file_exists {
            if options.never_overwrite {
                if options.verbose {
                    results.push(format!("  skipping: {} (file exists)", path));
                }
                skipped_count += 1;
                continue;
            }
            
            if options.freshen {
                // only extract if file exists (freshen mode)
                if !file_exists {
                    if options.verbose {
                        results.push(format!("  skipping: {} (freshen mode, file doesn't exist)", path));
                    }
                    skipped_count += 1;
                    continue;
                }
            }
            
            if !options.overwrite && !options.update && !options.freshen {
                if !options.quiet {
                    results.push(format!("  replace {}? [y]es, [n]o: n", extract_path));
                    results.push(format!("  skipping: {}", path));
                }
                skipped_count += 1;
                continue;
            }
        }

        // handle different file types
        if path.ends_with('/') {
            // directory entry
            if !options.junk_paths {
                ctx.create_dir_with_events(&extract_path)?;
                if options.verbose {
                    results.push(format!("  creating: {}", extract_path));
                }
            }
        } else if path.ends_with(".symlink") {
            // symlink entry
            let target = String::from_utf8_lossy(content);
            let link_path = extract_path.strip_suffix(".symlink").unwrap_or(&extract_path);
            
            // Ensure parent directories exist for symlinks
            ensure_parent_directories(ctx, link_path)?;
            
            ctx.create_symlink_with_events(link_path, &target)?;
            if options.verbose {
                results.push(format!("  linking: {} -> {}", link_path, target));
            }
            extracted_count += 1;
        } else {
            // regular file - ensure parent directories exist first
            ensure_parent_directories(ctx, &extract_path)?;
            
            ctx.create_file_with_events(&extract_path, content)?;
            
            if options.verbose {
                let action = if file_exists {
                    if options.update || options.freshen {
                        updated_count += 1;
                        "updating:"
                    } else {
                        "replacing:"
                    }
                } else {
                    "inflating:"
                };
                
                let compression_info = if *compressed_size != *original_size {
                    format!(" ({} -> {} bytes, {:.1}% compression)", 
                        compressed_size, original_size,
                        (1.0 - (*compressed_size as f32 / *original_size as f32)) * 100.0)
                } else {
                    " (stored)".to_string()
                };
                
                results.push(format!("  {} {}{}", action, extract_path, compression_info));
            }
            extracted_count += 1;
        }
    }

    if !options.quiet {
        let mut summary_parts = Vec::new();
        if extracted_count > 0 {
            summary_parts.push(format!("{} files extracted", extracted_count));
        }
        if updated_count > 0 {
            summary_parts.push(format!("{} files updated", updated_count));
        }
        if skipped_count > 0 {
            summary_parts.push(format!("{} files skipped", skipped_count));
        }
        
        if summary_parts.is_empty() {
            results.push("  no files processed".to_string());
        } else {
            results.push(format!("  {} to {}", summary_parts.join(", "), destination));
        }
    }

    Ok(results.join("\n"))
}

// check if file should be extracted based on patterns
fn should_extract_file(path: &str, options: &UnzipOptions) -> bool {
    // check file patterns (specific files requested)
    if !options.file_patterns.is_empty() {
        let matches_file_pattern = options.file_patterns.iter()
            .any(|pattern| matches_pattern(path, pattern, options.case_insensitive));
        if !matches_file_pattern {
            return false;
        }
    }

    // check exclude patterns
    for pattern in &options.exclude_patterns {
        if matches_pattern(path, pattern, options.case_insensitive) {
            return false;
        }
    }

    // check include patterns (if specified, file must match at least one)
    if !options.include_patterns.is_empty() {
        return options.include_patterns.iter()
            .any(|pattern| matches_pattern(path, pattern, options.case_insensitive));
    }

    true
}

// improved pattern matching with case sensitivity support
fn matches_pattern(text: &str, pattern: &str, case_insensitive: bool) -> bool {
    let text_to_match = if case_insensitive { text.to_lowercase() } else { text.to_string() };
    let pattern_to_match = if case_insensitive { pattern.to_lowercase() } else { pattern.to_string() };
    
    // convert glob pattern to regex
    let regex_pattern = pattern_to_match
        .replace(".", "\\.")
        .replace("*", ".*")
        .replace("?", ".")
        .replace("[", "\\[")
        .replace("]", "\\]");
    
    if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
        regex.is_match(&text_to_match)
    } else {
        // fallback to simple contains check
        text_to_match.contains(pattern_to_match.trim_matches('*'))
    }
}

// parse enhanced zip archive format
fn parse_zip_archive(content: &[u8]) -> Result<HashMap<String, (Vec<u8>, usize, usize)>, String> {
    let mut entries = HashMap::new();
    let mut cursor = 0;

    // check header
    if content.len() < 16 || &content[0..11] != b"ZIPARCHIVE\n" {
        return Err("unzip: not a valid zip archive or unsupported format".to_string());
    }
    cursor += 11;

    // read number of entries and compression level
    if cursor + 5 > content.len() {
        return Err("unzip: corrupted archive header".to_string());
    }
    let num_entries = u32::from_le_bytes([
        content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
    ]) as usize;
    cursor += 4;
    let _compression_level = content[cursor];
    cursor += 1;

    // read each entry
    for _ in 0..num_entries {
        // read path length
        if cursor + 4 > content.len() {
            return Err("unzip: corrupted archive entry".to_string());
        }
        let path_len = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;

        // read path
        if cursor + path_len > content.len() {
            return Err("unzip: corrupted archive path".to_string());
        }
        let path = String::from_utf8_lossy(&content[cursor..cursor+path_len]).to_string();
        cursor += path_len;

        // read original size and compressed size
        if cursor + 8 > content.len() {
            return Err("unzip: corrupted archive content lengths".to_string());
        }
        let original_size = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;
        let compressed_size = u32::from_le_bytes([
            content[cursor], content[cursor+1], content[cursor+2], content[cursor+3]
        ]) as usize;
        cursor += 4;

        // read and decompress content
        if cursor + compressed_size > content.len() {
            return Err("unzip: corrupted archive content".to_string());
        }
        let compressed_content = &content[cursor..cursor+compressed_size];
        let file_content = decompress_data(compressed_content);
        cursor += compressed_size;

        entries.insert(path, (file_content, original_size, compressed_size));
    }

    Ok(entries)
}

// decompress data (reverse of compression simulation)
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

// list archive contents with enhanced information
fn list_archive_contents(
    entries: &HashMap<String, (Vec<u8>, usize, usize)>, 
    archive_name: &str,
    options: &UnzipOptions
) -> CommandResult {
    let mut results = Vec::new();
    results.push(format!("Archive: {}", archive_name));
    
    if options.verbose {
        results.push(" Length   Method    Size  Cmpr    Date   Time   CRC-32   Name".to_string());
        results.push("--------  ------  ------- ---- ---------- ----- --------  ----".to_string());
    } else {
        results.push("  Length      Date    Time    Name".to_string());
        results.push("---------  ---------- -----   ----".to_string());
    }

    let mut total_original_size = 0;
    let mut total_compressed_size = 0;
    let mut files: Vec<_> = entries.iter().collect();
    files.sort_by_key(|(path, _)| path.as_str());

    for (path, (content, original_size, compressed_size)) in files {
        if !should_extract_file(path, options) {
            continue;
        }
        
        total_original_size += original_size;
        total_compressed_size += compressed_size;
        
        if options.verbose {
            let compression_method = if compressed_size == original_size {
                "Stored"
            } else if *compressed_size as f32 / *original_size as f32 > 0.8 {
                "Fast"
            } else if *compressed_size as f32 / *original_size as f32 > 0.6 {
                "Normal"
            } else {
                "Maximum"
            };
            
            let compression_ratio = if *original_size > 0 {
                (((*original_size - *compressed_size) as f32 / *original_size as f32) * 100.0) as u32
            } else {
                0
            };
            
            // simulate CRC32 for display
            let crc32 = content.iter().fold(0u32, |acc, &byte| acc.wrapping_add(byte as u32));
            
            results.push(format!("{:>8}  {:>6} {:>8} {:>3}% 1980-01-01 00:00 {:>8x}  {}",
                original_size, compression_method, compressed_size, compression_ratio, crc32, path));
        } else {
            results.push(format!("{:>9}  1980-01-01 00:00   {}", original_size, path));
        }
    }

    if options.verbose {
        results.push("--------          ------- ---                            -------".to_string());
        let total_compression = if total_original_size > 0 {
            ((total_original_size - total_compressed_size) as f32 / total_original_size as f32) * 100.0
        } else {
            0.0
        };
        results.push(format!("{:>8}          {:>7} {:>3.0}%                            {} files",
            total_original_size, total_compressed_size, total_compression, entries.len()));
    } else {
        results.push("---------                     -------".to_string());
        results.push(format!("{:>9}                     {} files", total_original_size, entries.len()));
    }

    Ok(results.join("\n"))
}

// test archive integrity
fn test_archive_integrity(
    entries: &HashMap<String, (Vec<u8>, usize, usize)>,
    archive_name: &str
) -> CommandResult {
    let mut results = Vec::new();
    results.push(format!("Archive: {}", archive_name));
    results.push("testing archive integrity...".to_string());
    
    let mut total_files = 0;
    let mut error_count = 0;
    
    let mut files: Vec<_> = entries.iter().collect();
    files.sort_by_key(|(path, _)| path.as_str());
    
    for (path, (content, original_size, _compressed_size)) in files {
        total_files += 1;
        
        // verify file integrity (check if decompression matches expected size)
        if content.len() != *original_size && !path.ends_with('/') {
            results.push(format!("  testing: {} ... ERROR (size mismatch)", path));
            error_count += 1;
        } else {
            results.push(format!("  testing: {} ... OK", path));
        }
    }
    
    if error_count == 0 {
        results.push(format!("archive integrity test passed: {} files verified", total_files));
    } else {
        results.push(format!("archive integrity test failed: {} errors in {} files", error_count, total_files));
    }
    
    Ok(results.join("\n"))
}

// Helper function to create parent directories recursively
fn ensure_parent_directories(ctx: &mut TerminalContext, file_path: &str) -> Result<(), String> {
    if let Some(parent_path) = std::path::Path::new(file_path).parent() {
        let parent_str = parent_path.to_string_lossy();
        if parent_str != "/" && !parent_str.is_empty() {
            // Split path into components and create each directory
            let components: Vec<&str> = parent_str.trim_matches('/').split('/').filter(|c| !c.is_empty()).collect();
            let mut current_path = String::new();
            
            for component in components {
                current_path = if current_path.is_empty() {
                    format!("/{}", component)
                } else {
                    format!("{}/{}", current_path, component)
                };
                
                // Only create if it doesn't exist
                if ctx.vfs.resolve_path(&current_path).is_none() {
                    ctx.create_dir_with_events(&current_path)?;
                }
            }
        }
    }
    Ok(())
} 
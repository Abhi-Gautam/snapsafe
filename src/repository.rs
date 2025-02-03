use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use chrono::{Local, DateTime};
use serde::{Serialize, Deserialize};

pub const REPO_FOLDER: &str = ".snapsafe";
pub const SNAPSHOTS_FOLDER: &str = "snapshots";
pub const CONFIG_FILE: &str = "base_path.txt";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const IGNORE_FILE: &str = ".snapsafeignore";

/// Structure to hold metadata for a single file.
#[derive(Serialize, Deserialize, Debug)]
pub struct FileMetadata {
    /// The file's path relative to the base directory.
    pub relative_path: String,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modification time as a formatted string.
    pub modified: String,
}

/// Read the ignore list from the .snapsafeignore file in the base directory.
/// Each non-empty, non-comment line is treated as an ignore pattern (literal file or directory name).
pub fn read_ignore_list(base: &Path) -> io::Result<Vec<String>> {
    let ignore_path = base.join(IGNORE_FILE);
    let mut ignore_list = Vec::new();

    if ignore_path.exists() {
        let file = fs::File::open(ignore_path)?;
        let reader = io::BufReader::new(file);
        for line_result in reader.lines() {
            let line = line_result?;
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                ignore_list.push(trimmed.to_string());
            }
        }
    }
    println!("DEBUG: Ignore list: {:?}", ignore_list);
    Ok(ignore_list)
}

/// Initialize the Snap Safe repository in the given base directory.
///
/// This creates a hidden folder `.snapsafe` and a subfolder `snapshots` inside it.
pub fn init_repository(base_dir: &str) -> io::Result<()> {
    let base_path = Path::new(base_dir);
    let repo_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repo_path.join(SNAPSHOTS_FOLDER);
    let config_path = repo_path.join(CONFIG_FILE);

    if repo_path.exists() {
        println!("Repository already exists at {:?}", repo_path);
    } else {
        fs::create_dir(&repo_path)?;
        println!("Created repository directory at {:?}", repo_path);
    }

    if snapshots_path.exists() {
        println!("Snapshots directory already exists at {:?}", snapshots_path);
    } else {
        fs::create_dir(&snapshots_path)?;
        println!("Created snapshots directory at {:?}", snapshots_path);
    }

    // Write the canonical base path into the config file.
    let canonical_base = fs::canonicalize(base_path)?;
    fs::write(&config_path, canonical_base.to_string_lossy().to_string())?;
    println!("Base path set to: {:?}", canonical_base);

    Ok(())
}

/// Search upward from the starting directory for a directory that contains the repository folder.
///
/// Returns the directory in which the repository (i.e. `.snapsafe`) is found.
pub fn find_repository_dir(start_dir: &Path) -> io::Result<PathBuf> {
    let mut current = start_dir;
    loop {
        let repo_path = current.join(REPO_FOLDER);
        if repo_path.exists() {
            return Ok(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Repository not found in current or parent directories",
    ))
}

/// Read the stored base directory from the repository config.
/// If not found, default to the canonical version of the given current_dir.
pub fn get_configured_base_dir(start_dir: &Path) -> io::Result<PathBuf> {
    let repo_base = find_repository_dir(start_dir)?;
    let repo_path = repo_base.join(REPO_FOLDER);
    let config_path = repo_path.join(CONFIG_FILE);
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        Ok(PathBuf::from(content.trim()))
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Repository not initialized. Please run the init command first.",
        ))
    }
}

/// Create a new snapshot of the base directory.
///
/// This function creates a new folder inside `.snapsafe/snapshots` with a timestamp as its name,
/// and recursively copies all files and directories from the base directory into it,
/// skipping the `.snapsafe` directory.
pub fn create_snapshot(message: Option<String>) -> io::Result<()> {
    let current_dir = std::env::current_dir()?;
    let base_path = get_configured_base_dir(&current_dir)?;
    let repo_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repo_path.join(SNAPSHOTS_FOLDER);
    let ignore_list = read_ignore_list(&base_path)?;

    // Ensure repository exists
    if !repo_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Repository not initialized. Please run 'init' command first."));
    }

    // Generate snapshot id using current timestamp
    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
    let snapshot_dir = snapshots_path.join(&timestamp);

    // Create snapshot directory
    fs::create_dir(&snapshot_dir)?;
    println!("Created snapshot directory: {:?}", snapshot_dir);

    // Optionally store the message (for now, just print it)
    if let Some(msg) = message {
        println!("Snapshot message: {}", msg);
        // Future work: Save the message in metadata.
    }

    // Prepare a vector to collect metadata for each file.
    let mut metadata_vec: Vec<FileMetadata> = Vec::new();

    // Copy files from base directory to snapshot directory, skipping the repository folder
    copy_dir_recursive(&base_path, &snapshot_dir, REPO_FOLDER, &base_path, &ignore_list, &mut metadata_vec)?;

    let manifest_path = snapshot_dir.join(MANIFEST_FILE);
    let manifest_json = serde_json::to_string_pretty(&metadata_vec)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(&manifest_path, manifest_json)?;

    println!("Snapshot created successfully.");

    Ok(())
}

/// Recursively copy contents from src to dst.
///
/// The `skip_dir` parameter specifies a directory name to skip (for example, the repository folder).
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf, skip_dir: &str, base: &Path, ignore_list: &Vec<String>, metadata: &mut Vec<FileMetadata>,) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip the directory we don't want to copy (like .snapsafe)
        if file_name_str == skip_dir {
            continue;
        }

        if ignore_list.contains(&file_name_str.to_string()) {
            println!("DEBUG: Ignoring {} as per .snapsafeignore", file_name_str);
            continue;
        }

        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            // Create the directory in the destination and recurse
            fs::create_dir_all(&dest_path)?;
            copy_dir_recursive(&path, &dest_path, skip_dir, base, ignore_list, metadata)?;
        } else if path.is_file() {
            // Copy the file
            fs::copy(&path, &dest_path)?;
            // Retrieve file metadata.
            let meta = fs::metadata(&path)?;
            let file_size = meta.len();

            // Get modification time as a formatted string.
            let modified_time: DateTime<Local> = meta.modified()
                .map(DateTime::<Local>::from)
                .unwrap_or_else(|_| Local::now());
            let modified_str = modified_time.format("%Y-%m-%d %H:%M:%S").to_string();

            // Compute the relative path (relative to the base directory).
            let relative_path = path.strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Create the FileMetadata entry.
            let file_meta = FileMetadata {
                relative_path,
                file_size,
                modified: modified_str,
            };
            metadata.push(file_meta);
        }
    }
    Ok(())
}
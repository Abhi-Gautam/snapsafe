use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use chrono::Local;

pub const REPO_FOLDER: &str = ".snapsafe";
pub const SNAPSHOTS_FOLDER: &str = "snapshots";
pub const CONFIG_FILE: &str = "base_path.txt";

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

    // Copy files from base directory to snapshot directory, skipping the repository folder
    copy_dir_recursive(&base_path, &snapshot_dir, REPO_FOLDER)?;

    println!("Snapshot created successfully.");

    Ok(())
}

/// Recursively copy contents from src to dst.
///
/// The `skip_dir` parameter specifies a directory name to skip (for example, the repository folder).
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf, skip_dir: &str) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip the directory we don't want to copy (like .snapsafe)
        if file_name_str == skip_dir {
            continue;
        }

        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            // Create the directory in the destination and recurse
            fs::create_dir_all(&dest_path)?;
            copy_dir_recursive(&path, &dest_path, skip_dir)?;
        } else if path.is_file() {
            // Copy the file
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
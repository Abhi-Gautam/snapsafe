use std::{fs, io};

use crate::{
    constants::{DEFAULT_IGNORE_ITEMS, IGNORE_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER},
    info, manifest,
};

/// Initializes the Snap Safe repository in the current directory.
/// This creates the hidden `.snapsafe` folder (and its subfolder for snapshots)
/// and initializes an empty head manifest.
pub fn init_repository() -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let repo_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repo_path.join(SNAPSHOTS_FOLDER);

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
    // Create .snapsafeignore file if it doesn't exist
    let ignore_path = base_path.join(IGNORE_FILE);
    if !ignore_path.exists() {
        // Get default ignore list from config
        let mut default_ignore_content = "# Default ignore patterns for Snap Safe\n".to_string();
        default_ignore_content
            .push_str("# Add file or directory names (one per line) to exclude from snapshots\n");

        // Add each item from the constants
        for &ignore_item in DEFAULT_IGNORE_ITEMS {
            default_ignore_content.push_str(&format!("{}\n", ignore_item));
        }

        fs::write(&ignore_path, default_ignore_content)?;
        println!("Created default {} file", IGNORE_FILE);
        println!(
            "You can edit this file to add patterns for files/folders to exclude from snapshots"
        );
        println!("Format: One filename or directory per line (similar to .gitignore)");
    }

    manifest::initialize_head_manifest(&base_path)?;

    println!("\nRepository initialized successfully!");
    println!("Run 'snapsafe snapshot -m \"Initial snapshot\"' to create your first snapshot");

    Ok(())
}

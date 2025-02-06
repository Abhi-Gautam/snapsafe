use std::{fs, io};

use crate::{constants::{HEAD_MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER}, manifest};

/// Initializes the Snap Safe repository in the current directory.
/// This creates the hidden `.snapsafe` folder (and its subfolder for snapshots)
/// and initializes an empty head manifest.
pub fn init_repository() -> io::Result<()> {
    let base_path = std::env::current_dir()?;

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

    // Initialize an empty head manifest if it does not exist.
    let head_manifest_path = repo_path.join(HEAD_MANIFEST_FILE);
    manifest::initialize_head_manifest(&head_manifest_path)?;
    Ok(())
}
use std::fs;
use std::io::{self, ErrorKind};

use crate::constants::{REPO_FOLDER, SNAPSHOTS_FOLDER};
use crate::info;
use crate::manifest::{self, load_head_manifest};
use crate::subcommands::snapshot;

/// Restores the contents of a snapshot to the working directory.
/// If no snapshot ID is provided, restores the latest snapshot.
/// If backup flag is true, creates a snapshot of the current state before restoring.
pub fn restore_snapshot(snapshot_id: Option<String>, backup: bool) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;

    if head_manifest.is_empty() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "No snapshots available to restore.",
        ));
    }

    // Determine which snapshot to restore (similar to diff.rs approach)
    let version = match snapshot_id {
        Some(id) => {
            // Check if the ID is "latest"
            if id.to_lowercase() == "latest" {
                head_manifest.last().unwrap().version.clone()
            } else {
                // Try exact match first
                let exact_match = head_manifest
                    .iter()
                    .find(|s| s.version == id)
                    .map(|s| s.version.clone());

                // If no exact match, try prefix match
                match exact_match {
                    Some(v) => v,
                    None => head_manifest
                        .iter()
                        .find(|s| s.version.starts_with(&id))
                        .map(|s| s.version.clone())
                        .ok_or_else(|| {
                            io::Error::new(
                                ErrorKind::NotFound,
                                format!("Snapshot {} not found", id),
                            )
                        })?,
                }
            }
        }
        None => {
            // If no ID provided, use the latest snapshot
            head_manifest.last().unwrap().version.clone()
        }
    };

    // If backup flag is set, take a snapshot of the current state
    if backup {
        println!("Creating backup snapshot before restoring...");
        if let Err(e) =
            snapshot::create_snapshot(Some("Auto-backup before restore".to_string()), None)
        {
            return Err(io::Error::new(
                ErrorKind::Other,
                format!("Failed to create backup snapshot: {}", e),
            ));
        }
        println!("Backup snapshot created successfully.");
    }

    // Get the path to the snapshot directory
    let snapshot_path = base_path
        .join(REPO_FOLDER)
        .join(SNAPSHOTS_FOLDER)
        .join(&version);

    if !snapshot_path.exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("Snapshot directory for {} not found", version),
        ));
    }

    // Load the snapshot manifest to get the file list
    let snap_option = manifest::load_snapshot_manifest(&base_path, &version)?;
    let (_, manifest) = snap_option.ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            format!("Manifest for snapshot {} not found", version),
        )
    })?;

    // Get the snapshot info from head manifest for display
    let snapshot_info = head_manifest.iter().find(|s| s.version == version).unwrap();

    println!("Restoring snapshot: {}", snapshot_info.version);
    println!("Created on: {}", snapshot_info.timestamp);
    if let Some(ref msg) = snapshot_info.message {
        println!("Message: {}", msg);
    }
    println!("This will overwrite files in your working directory. Press Enter to continue or Ctrl+C to abort...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Restore each file from the snapshot to the working directory
    for relative_path in manifest.keys() {
        let target_path = base_path.join(relative_path);
        let source_path = snapshot_path.join(relative_path);

        // Create parent directories if they don't exist
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy the file from the snapshot to the working directory
        if source_path.exists() && source_path.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }

    println!("Snapshot {} restored successfully.", version);
    Ok(())
}

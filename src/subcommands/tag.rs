use std::io;

use crate::info;
use crate::manifest::{load_head_manifest, save_head_manifest};
use crate::models::SnapshotMetadata;

/// Add, remove, or list tags for snapshots
pub fn manage_tags(
    snapshot_id: Option<String>,
    add: Option<Vec<String>>,
    remove: Option<Vec<String>>,
    list: bool,
) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let mut head_manifest = load_head_manifest(&base_path)?;

    let actual_id = info::resolve_snapshot_id(snapshot_id, &head_manifest)?;
    
    // Find the snapshot in the head manifest
    let snapshot_index = head_manifest
        .iter()
        .position(|s| s.version == actual_id || s.version.starts_with(&actual_id))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("Snapshot {} not found", actual_id)))?;
    
    // Add tags
    if let Some(ref tags) = add {  // Use ref to avoid moving tags
        // Reference to the snapshot
        let snapshot = &mut head_manifest[snapshot_index];
        
        // Initialize metadata if it doesn't exist
        if snapshot.metadata.is_none() {
            snapshot.metadata = Some(SnapshotMetadata::default());
        }
        
        let metadata = snapshot.metadata.as_mut().unwrap();
        
        for tag in tags {
            if !metadata.tags.contains(tag) {
                metadata.tags.push(tag.clone());
                println!("Added tag '{}' to snapshot {}", tag, snapshot.version);
            } else {
                println!("Tag '{}' already exists for snapshot {}", tag, snapshot.version);
            }
        }
        
        // Save the updated manifest
        save_head_manifest(&base_path, &head_manifest)?;
    }
    // Remove tags
    else if let Some(ref tags) = remove {  // Use ref to avoid moving tags
        // Reference to the snapshot
        let snapshot = &mut head_manifest[snapshot_index];
        
        // Initialize metadata if it doesn't exist
        if snapshot.metadata.is_none() {
            snapshot.metadata = Some(SnapshotMetadata::default());
        }
        
        let metadata = snapshot.metadata.as_mut().unwrap();
        
        for tag in tags {
            if let Some(pos) = metadata.tags.iter().position(|t| t == tag) {
                metadata.tags.remove(pos);
                println!("Removed tag '{}' from snapshot {}", tag, snapshot.version);
            } else {
                println!("Tag '{}' not found for snapshot {}", tag, snapshot.version);
            }
        }
        
        // Save the updated manifest
        save_head_manifest(&base_path, &head_manifest)?;
    }
    // List tags
    else if list || (add.is_none() && remove.is_none()) {
        // Use a separate binding for the snapshot to avoid borrow conflicts
        let snapshot = &head_manifest[snapshot_index];
        
        println!("Tags for snapshot {}:", snapshot.version);
        
        if let Some(ref metadata) = snapshot.metadata {
            if metadata.tags.is_empty() {
                println!("  No tags");
            } else {
                for tag in &metadata.tags {
                    println!("  - {}", tag);
                }
            }
        } else {
            println!("  No metadata available");
        }
    }
    
    Ok(())
}
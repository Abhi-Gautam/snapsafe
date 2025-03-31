use std::io;

use crate::info;
use crate::manifest::{load_head_manifest, save_head_manifest};

/// Add, update, remove, or list custom metadata for a snapshot
pub fn manage_metadata(
    snapshot_id: Option<String>,
    set: Option<Vec<String>>,
    remove: Option<String>,
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
    
    // Set custom metadata
    if let Some(ref values) = set {  // Use ref to avoid moving values
        if values.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Please provide exactly two values for --set: a key and a value."
            ));
        }
        
        let key = &values[0];
        let value = &values[1];
        
        // Reference to the snapshot (move after all pattern matches to avoid borrow conflicts)
        let snapshot = &mut head_manifest[snapshot_index];
        
        // Initialize metadata if it doesn't exist
        if snapshot.metadata.is_none() {
            snapshot.metadata = Some(crate::models::SnapshotMetadata::default());
        }
        
        let metadata = snapshot.metadata.as_mut().unwrap();
        
        metadata.custom.insert(key.clone(), value.clone());
        println!("Set metadata for snapshot {}: {} = {}", snapshot.version, key, value);
        
        // Save the updated manifest
        save_head_manifest(&base_path, &head_manifest)?;
    } 
    // Remove custom metadata
    else if let Some(ref key) = remove {  // Use ref to avoid moving key
        // Reference to the snapshot
        let snapshot = &mut head_manifest[snapshot_index];
        
        // Initialize metadata if it doesn't exist
        if snapshot.metadata.is_none() {
            snapshot.metadata = Some(crate::models::SnapshotMetadata::default());
        }
        
        let metadata = snapshot.metadata.as_mut().unwrap();
        
        if metadata.custom.remove(key).is_some() {
            println!("Removed metadata key '{}' from snapshot {}", key, snapshot.version);
        } else {
            println!("Metadata key '{}' not found for snapshot {}", key, snapshot.version);
        }
        
        // Save the updated manifest
        save_head_manifest(&base_path, &head_manifest)?;
    } 
    // List custom metadata
    else if list || (set.is_none() && remove.is_none()) {
        // Reference to the snapshot - using a separate binding to avoid borrow conflicts
        let snapshot = &head_manifest[snapshot_index];
        
        println!("Custom metadata for snapshot {}:", snapshot.version);
        
        if let Some(ref metadata) = snapshot.metadata {
            if metadata.custom.is_empty() {
                println!("  No custom metadata");
            } else {
                for (key, value) in &metadata.custom {
                    println!("  {} = {}", key, value);
                }
            }
        } else {
            println!("  No metadata available");
        }
    }
    
    Ok(())
}
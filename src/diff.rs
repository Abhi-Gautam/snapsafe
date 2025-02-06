use std::io;

use crate::{info::get_base_dir, manifest::{self, load_head_manifest}};

/// Diffs two snapshots identified by their version strings.
/// It prints the added, removed, and updated files in tabular form.
/// Only files that have differences (or are new/removed) are shown.
pub fn diff_snapshots(version1: String, version2: Option<String>) -> io::Result<()> {
    let (v1, v2) = get_snapshots_to_diff(version1, version2)?;
    let base_path = get_base_dir()?;
    
    // Load the detailed manifest for snapshot v1.
    let snap1_option = manifest::load_snapshot_manifest(&base_path, &v1)?;
    // Load the detailed manifest for snapshot v2.
    let snap2_option = manifest::load_snapshot_manifest(&base_path, &v2)?;
    
    // If either manifest is missing, return an error.
    let (_, manifest1) = snap1_option.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("Manifest for snapshot {} not found", v1))
    })?;
    let (_, manifest2) = snap2_option.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("Manifest for snapshot {} not found", v2))
    })?;
    // Determine added files: present in manifest2 but not in manifest1.
    let mut added: Vec<String> = Vec::new();
    // Determine removed files: present in manifest1 but not in manifest2.
    let mut removed: Vec<String> = Vec::new();
    // Determine updated files: present in both but with differences.
    let mut updated: Vec<String> = Vec::new();
    
    for (path, meta2) in &manifest2 {
        match manifest1.get(path.as_str()) {
            Some(meta1) => {
                if meta1.file_size != meta2.file_size || meta1.modified != meta2.modified {
                    updated.push(path.clone());
                }
            },
            None => {
                added.push(path.clone());
            }
        }
    }
    for path in manifest1.keys() {
        if !manifest2.contains_key(path) {
            removed.push(path.clone());
        }
    }
    
    // Print the diff in tabular form.
    if !added.is_empty() {
        println!("Added Files:");
        println!("{:-<50}", "");
        for file in &added {
            println!("{}", file);
        }
        println!("");
    }
    
    if !removed.is_empty() {
        println!("Removed Files:");
        println!("{:-<50}", "");
        for file in &removed {
            println!("{}", file);
        }
        println!("");
    }
    
    if !updated.is_empty() {
        println!("Updated Files:");
        println!("{:-<50}", "");
        for file in &updated {
            println!("{}", file);
        }
        println!("");
    }
    
    if added.is_empty() && removed.is_empty() && updated.is_empty() {
        println!("No differences found between snapshots {} and {}.", v1, v2);
    }
    
    Ok(())
}

/// Given a required snapshot version (version1) and an optional snapshot version (version2),
/// returns a tuple of snapshot versions to compare. If version2 is not provided,
/// it retrieves the latest snapshot version from the head manifest.
fn get_snapshots_to_diff(version1: String, version2: Option<String>) -> io::Result<(String, String)> {
    let base_path = get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;
    let v2 = match version2 {
        Some(v) => v,
        None => {
            if head_manifest.is_empty() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "No snapshots available for diff."));
            } else {
                head_manifest.last().unwrap().version.clone()
            }
        }
    };
    Ok((version1, v2))
}

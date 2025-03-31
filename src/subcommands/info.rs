use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::info;
use crate::manifest::{self, load_head_manifest};
use crate::models::FileMetadata;

/// Display detailed information about a specific snapshot
pub fn show_snapshot_info(snapshot_id: Option<String>) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;

    let actual_id = info::resolve_snapshot_id(snapshot_id, &head_manifest)?;

    // Find the snapshot in the head manifest
    let snapshot = head_manifest
        .iter()
        .find(|s| s.version == actual_id || s.version.starts_with(&actual_id))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Snapshot {} not found", actual_id),
            )
        })?;

    // Load the snapshot manifest
    let snap_option = manifest::load_snapshot_manifest(&base_path, &snapshot.version)?;
    let (_snapshot_dir, manifest) = snap_option.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest for snapshot {} not found", actual_id),
        )
    })?;

    // Calculate statistics
    let stats = calculate_snapshot_stats(&manifest);

    // Display the information
    println!("Snapshot Information");
    println!("===================");
    println!("Version:    {}", snapshot.version);
    println!("Created:    {}", snapshot.timestamp);
    if let Some(ref msg) = snapshot.message {
        println!("Message:    {}", msg);
    }
    println!();

    println!("Statistics");
    println!("==========");
    println!("Total files:       {}", stats.total_files);
    println!(
        "Total size:        {} bytes ({} MB)",
        stats.total_size,
        stats.total_size / 1024 / 1024
    );
    println!(
        "Largest file:      {} bytes ({})",
        stats.largest_file_size, stats.largest_file_path
    );
    println!("Average file size: {} bytes", stats.average_file_size);
    println!();

    println!("File Types");
    println!("==========");
    let mut file_types: Vec<(String, usize)> = stats.file_types.into_iter().collect();
    file_types.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count (descending)

    for (ext, count) in file_types.iter().take(10) {
        // Show top 10
        println!("{:<10} {}", ext, count);
    }

    Ok(())
}

/// Statistics about a snapshot
struct SnapshotStats {
    total_files: usize,
    total_size: u64,
    largest_file_size: u64,
    largest_file_path: String,
    average_file_size: u64,
    file_types: HashMap<String, usize>,
}

/// Calculate statistics about a snapshot
fn calculate_snapshot_stats(manifest: &HashMap<String, FileMetadata>) -> SnapshotStats {
    let total_files = manifest.len();
    let mut total_size = 0;
    let mut largest_file_size = 0;
    let mut largest_file_path = String::new();
    let mut file_types = HashMap::new();

    for (path, meta) in manifest {
        total_size += meta.file_size;

        if meta.file_size > largest_file_size {
            largest_file_size = meta.file_size;
            largest_file_path = path.clone();
        }

        // Extract file extension
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("no_ext")
            .to_string();

        *file_types.entry(ext).or_insert(0) += 1;
    }

    let average_file_size = if total_files > 0 {
        total_size / total_files as u64
    } else {
        0
    };

    SnapshotStats {
        total_files,
        total_size,
        largest_file_size,
        largest_file_path,
        average_file_size,
        file_types,
    }
}

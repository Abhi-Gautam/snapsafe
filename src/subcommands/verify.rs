use std::fs;
use std::io;
use std::path::Path;

use crate::constants::{MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER};
use crate::info;
use crate::manifest::load_head_manifest;
use crate::models::FileMetadata;

/// Verify the integrity of snapshots
pub fn verify_snapshots(snapshot_id: Option<String>) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;

    if head_manifest.is_empty() {
        println!("No snapshots found to verify.");
        return Ok(());
    }

    let snapshots_to_verify = if let Some(id) = snapshot_id {
        // Find the specific snapshot
        let snapshot = head_manifest
            .iter()
            .find(|s| s.version == id || s.version.starts_with(&id));

        match snapshot {
            Some(s) => vec![s.clone()],
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Snapshot {} not found", id),
                ));
            }
        }
    } else {
        // Verify all snapshots
        head_manifest
    };

    println!("Verifying {} snapshot(s)...", snapshots_to_verify.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for snapshot in &snapshots_to_verify {
        print!("Verifying snapshot {}: ", snapshot.version);

        match verify_single_snapshot(&base_path, &snapshot.version) {
            Ok(result) => {
                if result.success {
                    println!("✅ OK");
                    success_count += 1;
                } else {
                    println!("❌ FAILED");
                    println!("  Missing files: {}", result.missing_files);
                    println!("  Corrupt files: {}", result.corrupt_files);
                    error_count += 1;
                }
            }
            Err(e) => {
                println!("❌ ERROR: {}", e);
                error_count += 1;
            }
        }
    }

    println!("\nVerification complete:");
    println!("  Verified: {}", snapshots_to_verify.len());
    println!("  Success: {}", success_count);
    println!("  Failed: {}", error_count);

    if error_count > 0 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{} snapshot(s) failed verification", error_count),
        ));
    }

    Ok(())
}

/// Result of verifying a single snapshot
struct VerificationResult {
    success: bool,
    missing_files: usize,
    corrupt_files: usize,
}

/// Verify a single snapshot
fn verify_single_snapshot(base_path: &Path, version: &str) -> io::Result<VerificationResult> {
    let snapshot_path = base_path
        .join(REPO_FOLDER)
        .join(SNAPSHOTS_FOLDER)
        .join(version);

    if !snapshot_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Snapshot directory for {} not found", version),
        ));
    }

    let manifest_path = snapshot_path.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Manifest file for snapshot {} not found", version),
        ));
    }

    // Load the snapshot manifest
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let metadata_vec: Vec<FileMetadata> = serde_json::from_str(&manifest_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut missing_files = 0;
    let mut corrupt_files = 0;

    // Verify each file in the manifest
    for meta in &metadata_vec {
        let file_path = snapshot_path.join(&meta.relative_path);

        if !file_path.exists() {
            missing_files += 1;
            continue;
        }

        let actual_meta = match fs::metadata(&file_path) {
            Ok(m) => m,
            Err(_) => {
                corrupt_files += 1;
                continue;
            }
        };

        // Check file size
        if actual_meta.len() != meta.file_size {
            corrupt_files += 1;
        }
    }

    let success = missing_files == 0 && corrupt_files == 0;

    Ok(VerificationResult {
        success,
        missing_files,
        corrupt_files,
    })
}

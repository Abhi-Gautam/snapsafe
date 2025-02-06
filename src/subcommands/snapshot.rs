use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use chrono::{Local, DateTime};
use crate::constants::{IGNORE_FILE, MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER};
use crate::models::{SnapshotIndex, FileMetadata};
use crate::info;
use crate::manifest;

/// Creates a new snapshot using the current directory as the base.
/// The new snapshot folder name is determined by the versioning scheme (using an optional tag
/// or auto-incrementing from the last snapshot). Files are processed recursively;
/// if a file is unchanged compared to the previous snapshot (by size and modification time),
/// a hard link is created instead of copying. Detailed file metadata is collected and written
/// to a manifest file in the snapshot folder. The head manifest is updated with the new snapshot entry.
pub fn create_snapshot(message: Option<String>, tag: Option<String>) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let ignore_list = read_ignore_list(&base_path)?;

    let repo_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repo_path.join(SNAPSHOTS_FOLDER);

    if !repo_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Repository not initialized. Please run the init command first."));
    }

    // Load head manifest.
    let mut head_manifest = manifest::load_head_manifest(&base_path)?;
    // Determine new version string.
    let new_version = info::get_next_version(&head_manifest, tag);

    // New snapshot folder is named by the version.
    let snapshot_dir = snapshots_path.join(&new_version);
    fs::create_dir(&snapshot_dir)?;

    if let Some(ref msg) = message {
        println!("Snapshot message: {}", msg);
    }



    // Load previous snapshot manifest (if any) using the head manifest.
    let prev_snapshot = manifest::load_last_snapshot_manifest(&base_path, &head_manifest)?;

    // Prepare vector to collect detailed file metadata.
    let mut metadata_vec: Vec<FileMetadata> = Vec::new();
    copy_or_link_recursive_with_metadata(
        &base_path,
        &snapshot_dir,
        REPO_FOLDER,
        &base_path,
        &ignore_list,
        &prev_snapshot,
        &mut metadata_vec,
    )?;

    // Write the detailed manifest into the snapshot folder.
    let manifest_path = snapshot_dir.join(MANIFEST_FILE);
    let manifest_json = serde_json::to_string_pretty(&metadata_vec)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(&manifest_path, manifest_json)?;

    // Create a new snapshot index entry.
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let new_snapshot_index = SnapshotIndex {
        version: new_version.clone(),
        timestamp,
        message,
    };

    // Update the head manifest.
    head_manifest.push(new_snapshot_index);
    manifest::save_head_manifest(&base_path, &head_manifest)?;

    println!("Snapshot created successfully.");
    Ok(())
}


/// Reads the ignore list from the .snapsafeignore file in the base directory.
/// Each non-empty, non-comment line is treated as a literal file or directory name to ignore.
fn read_ignore_list(base: &Path) -> io::Result<Vec<String>> {
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
    Ok(ignore_list)
}

/// Recursively processes files and directories from src to dst, skipping entries that match skip_dir
/// or appear in ignore_list. For each file, if a previous snapshot exists and the file is unchanged
/// (based on size and modification time), an attempt is made to create a hard link from the previous
/// snapshot's file; otherwise, the file is copied. Collected file metadata is appended to the metadata vector.
fn copy_or_link_recursive_with_metadata(
    src: &Path,
    dst: &Path,
    skip_dir: &str,
    base: &Path,
    ignore_list: &Vec<String>,
    prev_snapshot: &Option<(PathBuf, HashMap<String, FileMetadata>)>,
    metadata: &mut Vec<FileMetadata>,
) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip the repository folder and entries in the ignore list.
        if file_name_str == skip_dir {
            continue;
        }
        if ignore_list.contains(&file_name_str.to_string()) {
            continue;
        }

        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_or_link_recursive_with_metadata(&path, &dest_path, skip_dir, base, ignore_list, prev_snapshot, metadata)?;
        } else if path.is_file() {
            let meta = fs::metadata(&path)?;
            let file_size = meta.len();
            let modified_time: DateTime<Local> = meta.modified()
                .map(DateTime::<Local>::from)
                .unwrap_or_else(|_| Local::now());
            let modified_str = modified_time.format("%Y-%m-%d %H:%M:%S").to_string();
            let relative_path = path.strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            let file_meta = FileMetadata {
                relative_path: relative_path.clone(),
                file_size,
                modified: modified_str.clone(),
            };

            let mut used_hard_link = false;
            if let Some((prev_snapshot_dir, prev_manifest)) = prev_snapshot {
                if let Some(prev_meta) = prev_manifest.get(&relative_path) {
                    if prev_meta.file_size == file_size && prev_meta.modified == modified_str {
                        let prev_file_path = prev_snapshot_dir.join(&relative_path);
                        match fs::hard_link(&prev_file_path, &dest_path) {
                            Ok(_) => {
                                used_hard_link = true;
                            },
                            Err(_) => {
                            }
                        }
                    }
                }
            }
            if !used_hard_link {
                fs::copy(&path, &dest_path)?;
            }
            metadata.push(file_meta);
        }
    }
    Ok(())
}

use std::{collections::HashMap, fs, io, path::{Path, PathBuf}};

use crate::{constants::{HEAD_MANIFEST_FILE, MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER}, models::{FileMetadata, SnapshotIndex}};

pub fn initialize_head_manifest(head_manifest_path: &Path) -> io::Result<()> {
    if !head_manifest_path.exists() {
        let empty: Vec<SnapshotIndex> = Vec::new();
        let manifest_json = serde_json::to_string_pretty(&empty)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(&head_manifest_path, manifest_json)?;
        println!("Initialized head manifest at {:?}", head_manifest_path);
    } else {
        println!("Head manifest already exists at {:?}", head_manifest_path);
    }
    Ok(())
}

/// Loads the head manifest from `.snapsafe/head_manifest.json`.
pub fn load_head_manifest(base_path: &Path) -> io::Result<Vec<SnapshotIndex>> {
    let head_manifest_path = base_path.join(REPO_FOLDER).join(HEAD_MANIFEST_FILE);
    if head_manifest_path.exists() {
        let content = fs::read_to_string(&head_manifest_path)?;
        let indices: Vec<SnapshotIndex> = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(indices)
    } else {
        Ok(Vec::new())
    }
}

/// Saves the head manifest to `.snapsafe/head_manifest.json`.
pub fn save_head_manifest(base_path: &Path, indices: &[SnapshotIndex]) -> io::Result<()> {
    let head_manifest_path = base_path.join(REPO_FOLDER).join(HEAD_MANIFEST_FILE);
    let json = serde_json::to_string_pretty(&indices)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(&head_manifest_path, json)?;
    Ok(())
}

/// Loads the previous snapshot's detailed manifest (if any) from the head manifest.
/// Returns an Option with a tuple containing the snapshot folder path and a HashMap
/// mapping each file's relative path to its FileMetadata.
pub fn load_prev_snapshot_manifest(base_path: &Path, head: &Vec<SnapshotIndex>) -> io::Result<Option<(PathBuf, HashMap<String, FileMetadata>)>> {
    if head.is_empty() {
        return Ok(None);
    }
    let last_entry = head.last().unwrap();
    let snapshot_folder = base_path.join(REPO_FOLDER).join(SNAPSHOTS_FOLDER).join(&last_entry.version);
    let manifest_path = snapshot_folder.join(MANIFEST_FILE);
    if manifest_path.exists() {
        let manifest_content = fs::read_to_string(&manifest_path)?;
        let metadata_vec: Vec<FileMetadata> = serde_json::from_str(&manifest_content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut metadata_map = HashMap::new();
        for meta in metadata_vec {
            metadata_map.insert(meta.relative_path.clone(), meta);
        }
        Ok(Some((snapshot_folder, metadata_map)))
    } else {
        Ok(None)
    }
}
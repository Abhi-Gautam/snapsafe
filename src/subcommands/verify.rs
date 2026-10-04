use crate::info;
use crate::integrity::hash_file_stable;
use crate::manifest::{self, load_head_manifest};
use crate::models::{FileKind, SnapshotIndex};
use crate::paths::join_relative;
use std::fs;
use std::io;
use std::path::Path;

pub fn verify_snapshots(snapshot_id: Option<String>) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;

    if head_manifest.is_empty() {
        println!("No snapshots found to verify.");
        return Ok(());
    }

    let snapshots = match snapshot_id {
        Some(id) => {
            let version = info::resolve_snapshot_id(Some(id), &head_manifest)?;
            let snapshot = head_manifest
                .iter()
                .find(|snapshot| snapshot.version == version)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "snapshot not found"))?;
            vec![snapshot]
        }
        None => head_manifest.iter().collect(),
    };

    println!("Verifying {} snapshot(s)...", snapshots.len());
    let mut failed = 0;

    for snapshot in &snapshots {
        match verify_snapshot(&base_path, &head_manifest, snapshot) {
            Ok(()) => println!("{}: OK", snapshot.version),
            Err(error) => {
                println!("{}: FAILED ({})", snapshot.version, error);
                failed += 1;
            }
        }
    }

    if failed > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} snapshot(s) failed verification", failed),
        ));
    }

    Ok(())
}

fn verify_snapshot(
    base_path: &Path,
    head_manifest: &[SnapshotIndex],
    snapshot: &SnapshotIndex,
) -> io::Result<()> {
    let loaded = manifest::load_snapshot_manifest(base_path, &snapshot.version)?;
    manifest::validate_loaded_manifest(&loaded, head_manifest, &snapshot.version)?;

    for file in loaded.files.values() {
        if file.kind == FileKind::Directory {
            let path = join_relative(&loaded.data_folder, &file.relative_path)?;
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not a directory", file.relative_path),
                ));
            }
            continue;
        }

        if matches!(
            file.kind,
            FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown
        ) {
            continue;
        }

        let path = join_relative(&loaded.data_folder, &file.relative_path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            io::Error::new(error.kind(), format!("{}: {}", file.relative_path, error))
        })?;

        if !metadata.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a regular file", file.relative_path),
            ));
        }

        if metadata.len() != file.file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} has an unexpected size", file.relative_path),
            ));
        }

        let actual = hash_file_stable(&path)?;
        if actual.content_hash != file.content_hash {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} has a content hash mismatch", file.relative_path),
            ));
        }
    }

    Ok(())
}

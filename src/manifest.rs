use crate::constants::{
    DATA_FOLDER, HEAD_MANIFEST_FILE, MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER,
};
use crate::integrity::{directory_hash, hash_bytes, validate_hash};
use crate::models::{
    FileKind, FileMetadata, HeadManifest, SnapshotIndex, SnapshotManifest, FORMAT_VERSION,
};
use crate::paths::{validate_relative_path, validate_snapshot_id};
use crate::repository::{ensure_real_directory, ensure_real_file};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct LoadedSnapshotManifest {
    pub snapshot_folder: PathBuf,
    pub data_folder: PathBuf,
    pub format_version: u32,
    pub ignore: Vec<String>,
    pub files: HashMap<String, FileMetadata>,
    pub manifest_hash: crate::models::ContentHash,
}

pub fn initialize_head_manifest(base_path: &Path) -> io::Result<()> {
    let head_manifest_path = base_path.join(REPO_FOLDER).join(HEAD_MANIFEST_FILE);
    if head_manifest_path.exists() {
        println!("Head manifest already exists at {:?}", head_manifest_path);
        return Ok(());
    }

    save_head_manifest(base_path, &[])?;
    println!("Initialized head manifest at {:?}", head_manifest_path);
    Ok(())
}

pub fn load_head_manifest(base_path: &Path) -> io::Result<Vec<SnapshotIndex>> {
    let path = base_path.join(REPO_FOLDER).join(HEAD_MANIFEST_FILE);
    ensure_real_file(&path, "head manifest")?;
    let bytes = fs::read(&path)?;
    let manifest: HeadManifest = serde_json::from_slice(&bytes).map_err(invalid_json)?;
    ensure_format_version(manifest.format_version, "head manifest")?;

    for snapshot in &manifest.snapshots {
        validate_snapshot_id(&snapshot.version)?;
        validate_hash(&snapshot.manifest_hash)?;
    }

    Ok(manifest.snapshots)
}

pub fn save_head_manifest(base_path: &Path, snapshots: &[SnapshotIndex]) -> io::Result<()> {
    let path = base_path.join(REPO_FOLDER).join(HEAD_MANIFEST_FILE);
    let manifest = HeadManifest {
        format_version: FORMAT_VERSION,
        snapshots: snapshots.to_vec(),
    };
    let bytes = serialize_json(&manifest)?;
    atomic_write(&path, &bytes)
}

pub fn write_snapshot_manifest(
    snapshot_folder: &Path,
    ignore: Vec<String>,
    files: Vec<FileMetadata>,
) -> io::Result<crate::models::ContentHash> {
    let manifest = SnapshotManifest {
        format_version: FORMAT_VERSION,
        ignore,
        files,
    };
    let bytes = serialize_json(&manifest)?;
    let manifest_hash = hash_bytes(&bytes);
    atomic_write(&snapshot_folder.join(MANIFEST_FILE), &bytes)?;
    Ok(manifest_hash)
}

pub fn load_snapshot_manifest(
    base_path: &Path,
    version: &str,
) -> io::Result<LoadedSnapshotManifest> {
    validate_snapshot_id(version)?;
    let snapshot_folder = base_path
        .join(REPO_FOLDER)
        .join(SNAPSHOTS_FOLDER)
        .join(version);
    ensure_real_directory(&snapshot_folder, "snapshot directory")?;
    let data_folder = snapshot_folder.join(DATA_FOLDER);
    ensure_real_directory(&data_folder, "snapshot data directory")?;
    let manifest_path = snapshot_folder.join(MANIFEST_FILE);
    ensure_real_file(&manifest_path, "snapshot manifest")?;
    let bytes = fs::read(&manifest_path)?;
    let manifest: SnapshotManifest = serde_json::from_slice(&bytes).map_err(invalid_json)?;
    ensure_format_version(manifest.format_version, "snapshot manifest")?;

    let mut files = HashMap::with_capacity(manifest.files.len());
    for file in manifest.files {
        validate_relative_path(&file.relative_path)?;
        validate_hash(&file.content_hash)?;
        match (&file.kind, &file.link_target) {
            (FileKind::File, None) => {}
            (FileKind::Directory, None) if file.content_hash == directory_hash() => {}
            (
                FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown,
                Some(target),
            ) if hash_bytes(target.as_bytes()) == file.content_hash => {}
            (
                FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown,
                Some(_),
            ) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("symlink hash mismatch for {}", file.relative_path),
                ));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid entry metadata for {}", file.relative_path),
                ));
            }
        }
        if files.insert(file.relative_path.clone(), file).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "snapshot manifest contains duplicate paths",
            ));
        }
    }

    Ok(LoadedSnapshotManifest {
        snapshot_folder,
        data_folder,
        format_version: manifest.format_version,
        ignore: manifest.ignore,
        files,
        manifest_hash: hash_bytes(&bytes),
    })
}

pub fn load_last_snapshot_manifest(
    base_path: &Path,
    head: &[SnapshotIndex],
) -> io::Result<Option<LoadedSnapshotManifest>> {
    match head.last() {
        Some(snapshot) => {
            let loaded = load_snapshot_manifest(base_path, &snapshot.version)?;
            validate_loaded_manifest(&loaded, head, &snapshot.version)?;
            Ok(Some(loaded))
        }
        None => Ok(None),
    }
}

pub fn validate_loaded_manifest(
    loaded: &LoadedSnapshotManifest,
    head: &[SnapshotIndex],
    version: &str,
) -> io::Result<()> {
    let snapshot = head
        .iter()
        .find(|snapshot| snapshot.version == version)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("snapshot {} is not present in the head manifest", version),
            )
        })?;

    if loaded.manifest_hash != snapshot.manifest_hash {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("manifest hash mismatch for snapshot {}", version),
        ));
    }

    Ok(())
}

fn ensure_format_version(version: u32, label: &str) -> io::Result<()> {
    if version != FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "unsupported {} format version {}; expected {}",
                label, version, FORMAT_VERSION
            ),
        ));
    }
    Ok(())
}

pub(crate) fn serialize_json<T: serde::Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(invalid_json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid manifest path"))?;
    let temporary_path = path.with_file_name(format!(".{}.{}.tmp", file_name, std::process::id()));

    let mut file = fs::File::create(&temporary_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    if let Err(error) = replace_path(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    sync_parent(path)
}

#[cfg(not(windows))]
fn replace_path(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_path(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn invalid_json(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::ensure_format_version;
    use crate::models::FORMAT_VERSION;

    #[test]
    fn rejects_other_format_versions() {
        assert!(ensure_format_version(FORMAT_VERSION, "manifest").is_ok());
        assert!(ensure_format_version(FORMAT_VERSION + 1, "manifest").is_err());
    }
}

use crate::constants::{
    DATA_FOLDER, HEAD_MANIFEST_FILE, IGNORE_FILE, MANIFEST_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER,
    TEMP_FOLDER,
};
use crate::integrity::{directory_hash, hash_bytes, hash_file_stable, validate_hash};
use crate::manifest;
use crate::models::{
    ContentHash, FileKind, FileMetadata, SnapshotIndex, SnapshotManifest, SnapshotMetadata,
    FORMAT_VERSION,
};
use crate::paths::{join_relative, relative_path, validate_relative_path, validate_snapshot_id};
use crate::repository::{
    ensure_layout, ensure_real_directory, ensure_real_file, recover_transactions, remove_tree,
    RepositoryLock,
};
use chrono::{DateTime, Local, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION_FOLDER: &str = "migration-v1-v2";
const V2_SIDECAR: &str = "manifest.v2";
const V1_BACKUP: &str = "manifest.v1";
const PREPARED_DATA_FOLDER: &str = "data";

pub fn prepare_current_repository() -> io::Result<()> {
    let base_path = std::env::current_dir()?;
    let repository_path = base_path.join(REPO_FOLDER);
    ensure_real_directory(&repository_path, "repository")?;
    let _lock = RepositoryLock::acquire(&base_path)?;

    match detect_head_format(&repository_path.join(HEAD_MANIFEST_FILE))? {
        HeadFormat::Current => {
            ensure_layout(&base_path)?;
            let head = manifest::load_head_manifest(&base_path)?;
            recover_transactions(&base_path, &head)?;
            if migration_root(&base_path).exists() {
                validate_current_repository(&base_path, &head)?;
                protect_migrated_files(&base_path, &head)?;
                finalize_migration(&base_path, &head)?;
            }
        }
        HeadFormat::Legacy => migrate_repository(&base_path)?,
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadFormat {
    Legacy,
    Current,
}

fn detect_head_format(path: &Path) -> io::Result<HeadFormat> {
    ensure_real_file(path, "head manifest")?;
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path)?).map_err(invalid_data)?;
    match value {
        serde_json::Value::Array(_) => Ok(HeadFormat::Legacy),
        serde_json::Value::Object(object)
            if object
                .get("format_version")
                .and_then(|value| value.as_u64())
                == Some(FORMAT_VERSION as u64) =>
        {
            Ok(HeadFormat::Current)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported head manifest format",
        )),
    }
}

fn migrate_repository(base_path: &Path) -> io::Result<()> {
    let repository_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repository_path.join(SNAPSHOTS_FOLDER);
    ensure_real_directory(&snapshots_path, "snapshots directory")?;

    let temporary_path = repository_path.join(TEMP_FOLDER);
    match fs::symlink_metadata(&temporary_path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "temporary path is not a real directory",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&temporary_path)?,
        Err(error) => return Err(error),
    }

    let legacy_head: Vec<LegacySnapshotIndex> =
        serde_json::from_slice(&fs::read(repository_path.join(HEAD_MANIFEST_FILE))?)
            .map_err(invalid_data)?;
    validate_legacy_head(&legacy_head)?;

    println!(
        "Migrating {} snapshot(s) to repository format {}...",
        legacy_head.len(),
        FORMAT_VERSION
    );

    let ignore = read_ignore_list(base_path)?;
    let migration_root = temporary_path.join(MIGRATION_FOLDER);
    fs::create_dir_all(&migration_root)?;
    let mut hashes = HashMap::new();
    let mut migrated_head = Vec::with_capacity(legacy_head.len());
    let mut resumed_snapshots = HashSet::new();

    for (index, legacy_snapshot) in legacy_head.iter().enumerate() {
        let snapshot_folder = join_relative(&snapshots_path, &legacy_snapshot.version)?;
        ensure_real_directory(&snapshot_folder, "snapshot directory")?;
        let snapshot_migration = join_relative(&migration_root, &legacy_snapshot.version)?;
        fs::create_dir_all(&snapshot_migration)?;
        let prepared =
            prepare_snapshot_manifest(&snapshot_folder, &snapshot_migration, &ignore, &mut hashes)?;
        if prepared.requires_validation {
            resumed_snapshots.insert(legacy_snapshot.version.clone());
        }
        migrated_head.push(SnapshotIndex {
            version: legacy_snapshot.version.clone(),
            timestamp: migrate_snapshot_timestamp(&legacy_snapshot.timestamp)?,
            message: legacy_snapshot.message.clone(),
            manifest_hash: prepared.manifest_hash,
            metadata: legacy_snapshot.metadata.clone(),
        });
        println!(
            "[{}/{}] {}",
            index + 1,
            legacy_head.len(),
            legacy_snapshot.version
        );
    }

    for snapshot in &migrated_head {
        commit_snapshot_manifest(
            &snapshots_path,
            &migration_root,
            &snapshot.version,
            resumed_snapshots.contains(&snapshot.version),
        )?;
    }
    manifest::save_head_manifest(base_path, &migrated_head)?;

    validate_current_repository(base_path, &migrated_head)?;
    protect_migrated_files(base_path, &migrated_head)?;
    finalize_migration(base_path, &migrated_head)?;
    recover_transactions(base_path, &migrated_head)?;
    println!("Repository migration complete.");
    Ok(())
}

struct PreparedManifest {
    manifest_hash: ContentHash,
    requires_validation: bool,
}

fn prepare_snapshot_manifest(
    snapshot_folder: &Path,
    migration_folder: &Path,
    ignore: &[String],
    hashes: &mut HashMap<PhysicalIdentity, ContentHash>,
) -> io::Result<PreparedManifest> {
    let manifest_path = snapshot_folder.join(MANIFEST_FILE);
    let sidecar_path = migration_folder.join(V2_SIDECAR);
    let backup_path = migration_folder.join(V1_BACKUP);
    let prepared_data = migration_folder.join(PREPARED_DATA_FOLDER);

    if is_current_manifest(&manifest_path)? {
        let bytes = fs::read(&manifest_path)?;
        let current = parse_current_manifest(&bytes)?;
        seed_hash_cache(&snapshot_folder.join(DATA_FOLDER), &current, hashes)?;
        return Ok(PreparedManifest {
            manifest_hash: hash_bytes(&bytes),
            requires_validation: false,
        });
    }

    if sidecar_path.exists() {
        ensure_real_file(&sidecar_path, "prepared manifest")?;
        let bytes = fs::read(&sidecar_path)?;
        let current = parse_current_manifest(&bytes)?;
        let data_source = if prepared_data.is_dir() {
            prepared_data.clone()
        } else {
            snapshot_folder.join(DATA_FOLDER)
        };
        seed_hash_cache(&data_source, &current, hashes)?;
        return Ok(PreparedManifest {
            manifest_hash: hash_bytes(&bytes),
            requires_validation: true,
        });
    }

    ensure_real_file(&manifest_path, "legacy snapshot manifest")?;
    let legacy_bytes = fs::read(&manifest_path)?;
    if !backup_path.exists() {
        manifest::atomic_write(&backup_path, &legacy_bytes)?;
    }
    let legacy_files: Vec<LegacyFileMetadata> =
        serde_json::from_slice(&legacy_bytes).map_err(invalid_data)?;
    let files = convert_legacy_files(snapshot_folder, legacy_files, hashes)?;
    prepare_migrated_data(snapshot_folder, &prepared_data, &files)?;
    let current = SnapshotManifest {
        format_version: FORMAT_VERSION,
        ignore: migration_ignore_rules(ignore, &files),
        files,
    };
    let bytes = manifest::serialize_json(&current)?;
    let manifest_hash = hash_bytes(&bytes);
    manifest::atomic_write(&sidecar_path, &bytes)?;
    Ok(PreparedManifest {
        manifest_hash,
        requires_validation: false,
    })
}

fn convert_legacy_files(
    snapshot_folder: &Path,
    legacy_files: Vec<LegacyFileMetadata>,
    hashes: &mut HashMap<PhysicalIdentity, ContentHash>,
) -> io::Result<Vec<FileMetadata>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    for legacy in legacy_files {
        let relative = normalize_legacy_path(&legacy.relative_path)?;
        if relative == IGNORE_FILE || relative.starts_with(&format!("{}/", REPO_FOLDER)) {
            continue;
        }
        if !seen.insert(relative.clone()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("duplicate legacy path: {}", relative),
            ));
        }

        let path = join_relative(snapshot_folder, &relative)?;
        ensure_real_file(&path, "legacy snapshot file")?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() != legacy.file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("legacy snapshot size mismatch: {}", relative),
            ));
        }

        let identity = physical_identity(&path, &metadata)?;
        let content_hash = match hashes.get(&identity) {
            Some(hash) => hash.clone(),
            None => {
                let captured = hash_file_stable(&path)?;
                hashes.insert(identity, captured.content_hash.clone());
                captured.content_hash
            }
        };
        let mode = unix_mode(&path)?;
        let modified: DateTime<Utc> = metadata.modified()?.into();
        files.push(FileMetadata {
            relative_path: relative,
            file_size: metadata.len(),
            modified: modified.to_rfc3339_opts(SecondsFormat::Nanos, true),
            kind: FileKind::File,
            content_hash,
            link_target: None,
            unix_mode: mode,
        });
    }

    collect_legacy_directories(snapshot_folder, snapshot_folder, &mut seen, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn migration_ignore_rules(ignore: &[String], files: &[FileMetadata]) -> Vec<String> {
    let represented_names: HashSet<&str> = files
        .iter()
        .filter_map(|file| file.relative_path.rsplit('/').next())
        .collect();
    ignore
        .iter()
        .filter(|item| !represented_names.contains(item.as_str()))
        .cloned()
        .collect()
}

fn prepare_migrated_data(
    snapshot_folder: &Path,
    prepared_data: &Path,
    files: &[FileMetadata],
) -> io::Result<()> {
    if prepared_data.exists() {
        remove_tree(prepared_data)?;
    }
    fs::create_dir_all(prepared_data)?;

    let mut directories: Vec<_> = files
        .iter()
        .filter(|file| file.kind == FileKind::Directory)
        .collect();
    directories.sort_by_key(|file| file.relative_path.matches('/').count());
    for directory in directories {
        fs::create_dir_all(join_relative(prepared_data, &directory.relative_path)?)?;
    }

    for file in files.iter().filter(|file| file.kind == FileKind::File) {
        let source = join_relative(snapshot_folder, &file.relative_path)?;
        let destination = join_relative(prepared_data, &file.relative_path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::hard_link(source, destination)?;
    }
    Ok(())
}

fn collect_legacy_directories(
    snapshot_folder: &Path,
    directory: &Path,
    seen: &mut HashSet<String>,
    files: &mut Vec<FileMetadata>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if directory == snapshot_folder && name.to_str() == Some(MANIFEST_FILE) {
            continue;
        }

        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let relative = relative_path(snapshot_folder, &path)?;
            if seen.insert(relative.clone()) {
                let metadata = fs::symlink_metadata(&path)?;
                let modified: DateTime<Utc> = metadata.modified()?.into();
                files.push(FileMetadata {
                    relative_path: relative,
                    file_size: 0,
                    modified: modified.to_rfc3339_opts(SecondsFormat::Nanos, true),
                    kind: FileKind::Directory,
                    content_hash: directory_hash(),
                    link_target: None,
                    unix_mode: unix_mode(&path)?,
                });
            }
            collect_legacy_directories(snapshot_folder, &path, seen, files)?;
        } else if file_type.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected symlink in legacy snapshot: {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn commit_snapshot_manifest(
    snapshots_path: &Path,
    migration_root: &Path,
    version: &str,
    validate_prepared: bool,
) -> io::Result<()> {
    let folder = join_relative(snapshots_path, version)?;
    let migration_folder = join_relative(migration_root, version)?;
    let manifest_path = folder.join(MANIFEST_FILE);
    let sidecar_path = migration_folder.join(V2_SIDECAR);
    let prepared_data = migration_folder.join(PREPARED_DATA_FOLDER);
    let data_folder = folder.join(DATA_FOLDER);

    ensure_real_file(&sidecar_path, "prepared manifest")?;
    let bytes = fs::read(&sidecar_path)?;
    let current = parse_current_manifest(&bytes)?;

    if prepared_data.exists() {
        ensure_real_directory(&prepared_data, "prepared snapshot data")?;
        if validate_prepared {
            validate_migrated_data(&prepared_data, &current)?;
        }
        remove_legacy_payload(&folder, &current)?;
        fs::rename(&prepared_data, &data_folder)?;
    } else {
        ensure_real_directory(&data_folder, "migrated snapshot data")?;
        validate_migrated_data(&data_folder, &current)?;
    }

    if !is_current_manifest(&manifest_path)? {
        manifest::atomic_write(&manifest_path, &bytes)?;
    }
    Ok(())
}

fn validate_migrated_data(data_folder: &Path, current: &SnapshotManifest) -> io::Result<()> {
    for entry in &current.files {
        let path = join_relative(data_folder, &entry.relative_path)?;
        match entry.kind {
            FileKind::File => {
                ensure_real_file(&path, "migrated snapshot file")?;
                let captured = hash_file_stable(&path)?;
                if captured.size != entry.file_size || captured.content_hash != entry.content_hash {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("migrated snapshot file mismatch: {}", entry.relative_path),
                    ));
                }
            }
            FileKind::Directory => {
                ensure_real_directory(&path, "migrated snapshot directory")?;
            }
            FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "legacy migration produced an unexpected symlink",
                ));
            }
        }
    }
    Ok(())
}

fn remove_legacy_payload(snapshot_folder: &Path, current: &SnapshotManifest) -> io::Result<()> {
    for file in current
        .files
        .iter()
        .filter(|file| file.kind == FileKind::File)
    {
        let path = join_relative(snapshot_folder, &file.relative_path)?;
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    let mut directories: Vec<_> = current
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Directory)
        .collect();
    directories.sort_by_key(|file| std::cmp::Reverse(file.relative_path.matches('/').count()));
    for directory in directories {
        let path = join_relative(snapshot_folder, &directory.relative_path)?;
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn validate_current_repository(base_path: &Path, head: &[SnapshotIndex]) -> io::Result<()> {
    for snapshot in head {
        let loaded = manifest::load_snapshot_manifest(base_path, &snapshot.version)?;
        manifest::validate_loaded_manifest(&loaded, head, &snapshot.version)?;
    }
    Ok(())
}

fn protect_migrated_files(base_path: &Path, head: &[SnapshotIndex]) -> io::Result<()> {
    for snapshot in head {
        let loaded = manifest::load_snapshot_manifest(base_path, &snapshot.version)?;
        for file in loaded.files.values() {
            if file.kind == FileKind::File {
                let path = join_relative(&loaded.data_folder, &file.relative_path)?;
                make_snapshot_file_readonly(&path)?;
            }
        }
    }
    Ok(())
}

fn migration_root(base_path: &Path) -> PathBuf {
    base_path
        .join(REPO_FOLDER)
        .join(TEMP_FOLDER)
        .join(MIGRATION_FOLDER)
}

fn finalize_migration(base_path: &Path, _head: &[SnapshotIndex]) -> io::Result<()> {
    let migration_root = migration_root(base_path);
    if migration_root.exists() {
        remove_tree(&migration_root)?;
    }
    Ok(())
}

fn parse_current_manifest(bytes: &[u8]) -> io::Result<SnapshotManifest> {
    let manifest: SnapshotManifest = serde_json::from_slice(bytes).map_err(invalid_data)?;
    if manifest.format_version != FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported prepared manifest version",
        ));
    }
    for file in &manifest.files {
        validate_relative_path(&file.relative_path)?;
        validate_hash(&file.content_hash)?;
    }
    Ok(manifest)
}

fn is_current_manifest(path: &Path) -> io::Result<bool> {
    match fs::read(path) {
        Ok(bytes) => {
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(invalid_data)?;
            Ok(value.get("format_version").and_then(|value| value.as_u64())
                == Some(FORMAT_VERSION as u64))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn seed_hash_cache(
    snapshot_folder: &Path,
    manifest: &SnapshotManifest,
    hashes: &mut HashMap<PhysicalIdentity, ContentHash>,
) -> io::Result<()> {
    for file in &manifest.files {
        if file.kind != FileKind::File {
            continue;
        }
        let path = join_relative(snapshot_folder, &file.relative_path)?;
        let metadata = fs::metadata(&path)?;
        hashes.insert(
            physical_identity(&path, &metadata)?,
            file.content_hash.clone(),
        );
    }
    Ok(())
}

fn validate_legacy_head(head: &[LegacySnapshotIndex]) -> io::Result<()> {
    let mut versions = HashSet::new();
    for snapshot in head {
        validate_snapshot_id(&snapshot.version)?;
        if !versions.insert(&snapshot.version) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("duplicate snapshot version: {}", snapshot.version),
            ));
        }
    }
    Ok(())
}

fn normalize_legacy_path(path: &str) -> io::Result<String> {
    let portable = path.replace('\\', "/");
    let path = Path::new(&portable);
    if path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "legacy manifest contains an absolute path",
        ));
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 path"))?,
            ),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "legacy manifest contains an unsafe path",
                ));
            }
        }
    }
    let normalized = parts.join("/");
    validate_relative_path(&normalized)?;
    Ok(normalized)
}

fn migrate_snapshot_timestamp(timestamp: &str) -> io::Result<String> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp) {
        return Ok(timestamp
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Nanos, true));
    }

    let naive =
        NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").map_err(invalid_data)?;
    let local = Local
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid legacy timestamp"))?;
    Ok(local
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn read_ignore_list(base_path: &Path) -> io::Result<Vec<String>> {
    let path = base_path.join(IGNORE_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for line in io::BufReader::new(fs::File::open(path)?).lines() {
        let line = line?;
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            items.push(line.to_string());
        }
    }
    Ok(items)
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct PhysicalIdentity {
    handle: same_file::Handle,
    size: u64,
    modified_before_epoch: bool,
    modified_seconds: u64,
    modified_nanos: u32,
}

fn physical_identity(path: &Path, metadata: &fs::Metadata) -> io::Result<PhysicalIdentity> {
    let (before, seconds, nanos) = system_time_parts(metadata.modified()?);
    Ok(PhysicalIdentity {
        handle: same_file::Handle::from_path(path)?,
        size: metadata.len(),
        modified_before_epoch: before,
        modified_seconds: seconds,
        modified_nanos: nanos,
    })
}

fn system_time_parts(time: SystemTime) -> (bool, u64, u32) {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => (false, duration.as_secs(), duration.subsec_nanos()),
        Err(error) => {
            let duration = error.duration();
            (true, duration.as_secs(), duration.subsec_nanos())
        }
    }
}

#[cfg(unix)]
fn unix_mode(path: &Path) -> io::Result<Option<u32>> {
    use std::os::unix::fs::PermissionsExt;
    Ok(Some(fs::metadata(path)?.permissions().mode()))
}

#[cfg(not(unix))]
fn unix_mode(_path: &Path) -> io::Result<Option<u32>> {
    Ok(None)
}

#[cfg(unix)]
fn make_snapshot_file_readonly(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}

#[cfg(windows)]
fn make_snapshot_file_readonly(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}

#[cfg(not(any(unix, windows)))]
fn make_snapshot_file_readonly(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[derive(Debug, Deserialize)]
struct LegacyFileMetadata {
    relative_path: String,
    file_size: u64,
    #[serde(rename = "modified")]
    _modified: String,
}

#[derive(Debug, Deserialize)]
struct LegacySnapshotIndex {
    version: String,
    timestamp: String,
    message: Option<String>,
    #[serde(default)]
    metadata: Option<SnapshotMetadata>,
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

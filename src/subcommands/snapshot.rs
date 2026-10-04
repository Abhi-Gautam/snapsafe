use crate::constants::{DATA_FOLDER, IGNORE_FILE, REPO_FOLDER, SNAPSHOTS_FOLDER, TEMP_FOLDER};
use crate::info;
use crate::integrity::{
    copy_file_stable, directory_hash, hash_bytes, hash_file_stable, CapturedFile,
};
use crate::manifest::{self, LoadedSnapshotManifest};
use crate::models::{FileKind, FileMetadata, SnapshotIndex, SnapshotMetadata};
use crate::paths::{join_relative, relative_path};
use crate::repository::{ensure_layout, recover_transactions, RepositoryLock};
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Debug, Default)]
pub struct SnapshotOptions {
    pub message: Option<String>,
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub custom_metadata: HashMap<String, String>,
    pub ignore: Option<Vec<String>>,
}

pub fn create_snapshot(options: SnapshotOptions) -> io::Result<String> {
    let base_path = info::get_base_dir()?;
    let _lock = RepositoryLock::acquire(&base_path)?;
    create_snapshot_unlocked(options)
}

pub(crate) fn create_snapshot_unlocked(options: SnapshotOptions) -> io::Result<String> {
    let base_path = info::get_base_dir()?;
    ensure_layout(&base_path)?;
    let ignore_list = match options.ignore.clone() {
        Some(ignore) => ignore,
        None => read_ignore_list(&base_path)?,
    };
    let repo_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repo_path.join(SNAPSHOTS_FOLDER);
    let temp_path = repo_path.join(TEMP_FOLDER);

    let existing_head = manifest::load_head_manifest(&base_path)?;
    recover_transactions(&base_path, &existing_head)?;
    let mut head_manifest = manifest::load_head_manifest(&base_path)?;
    let new_version = info::get_next_version(&head_manifest, options.version.clone())?;

    if head_manifest
        .iter()
        .any(|snapshot| snapshot.version == new_version)
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("snapshot {} already exists", new_version),
        ));
    }

    let snapshot_dir = snapshots_path.join(&new_version);
    if snapshot_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "snapshot directory {} already exists",
                snapshot_dir.display()
            ),
        ));
    }

    let staging_dir = temp_path.join(format!(
        "snapshot-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir(&staging_dir)?;
    let staging_data = staging_dir.join(DATA_FOLDER);
    fs::create_dir(&staging_data)?;
    let staging_name = staging_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid staging path"))?;
    let pending_path = temp_path.join(format!("snapshot-{}.pending", new_version));

    if let Some(message) = &options.message {
        println!("Snapshot message: {}", message);
    }

    let creation_result = (|| {
        let previous = manifest::load_last_snapshot_manifest(&base_path, &head_manifest)?;
        let mut files = Vec::new();
        capture_directory(
            &base_path,
            &staging_data,
            &base_path,
            &ignore_list,
            previous.as_ref(),
            &mut files,
        )?;

        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        let manifest_hash =
            manifest::write_snapshot_manifest(&staging_dir, ignore_list.clone(), files)?;
        let mut marker = fs::File::create(&pending_path)?;
        writeln!(marker, "{}", new_version)?;
        writeln!(marker, "{}", staging_name)?;
        marker.sync_all()?;
        fs::rename(&staging_dir, &snapshot_dir)?;

        let metadata = if options.tags.is_empty() && options.custom_metadata.is_empty() {
            None
        } else {
            Some(SnapshotMetadata {
                tags: options.tags,
                custom: options.custom_metadata,
            })
        };

        head_manifest.push(SnapshotIndex {
            version: new_version.clone(),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            message: options.message,
            manifest_hash,
            metadata,
        });

        if let Err(error) = manifest::save_head_manifest(&base_path, &head_manifest) {
            let _ = fs::remove_dir_all(&snapshot_dir);
            return Err(error);
        }
        let _ = fs::remove_file(&pending_path);

        Ok::<(), io::Error>(())
    })();

    if let Err(error) = creation_result {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_file(&pending_path);
        return Err(error);
    }

    println!("Snapshot {} created successfully.", new_version);
    Ok(new_version)
}

fn capture_directory(
    source_dir: &Path,
    destination_dir: &Path,
    base_path: &Path,
    ignore_list: &[String],
    previous: Option<&LoadedSnapshotManifest>,
    files: &mut Vec<FileMetadata>,
) -> io::Result<()> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 path"))?;

        if file_name == REPO_FOLDER
            || file_name == IGNORE_FILE
            || ignore_list.iter().any(|item| item == &file_name)
        {
            continue;
        }

        let destination_path = destination_dir.join(&file_name);
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            fs::create_dir(&destination_path)?;
            let directory_metadata = capture_directory_metadata(base_path, &path)?;
            capture_directory(
                &path,
                &destination_path,
                base_path,
                ignore_list,
                previous,
                files,
            )?;
            files.push(directory_metadata);
        } else if file_type.is_file() {
            let relative_path = relative_path(base_path, &path)?;

            let previous_file = previous.and_then(|manifest| manifest.files.get(&relative_path));
            let captured = capture_file(
                &path,
                &destination_path,
                &relative_path,
                previous,
                previous_file,
            )?;

            protect_snapshot_file(&destination_path)?;
            files.push(FileMetadata {
                relative_path,
                file_size: captured.size,
                modified: captured.modified,
                kind: FileKind::File,
                content_hash: captured.content_hash,
                link_target: None,
                unix_mode: unix_mode(&path)?,
            });
        } else if file_type.is_symlink() {
            files.push(capture_symlink(base_path, &path)?);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported filesystem entry: {}", path.display()),
            ));
        }
    }

    Ok(())
}

fn capture_directory_metadata(base_path: &Path, path: &Path) -> io::Result<FileMetadata> {
    let metadata = fs::symlink_metadata(path)?;
    let modified: DateTime<Utc> = metadata.modified()?.into();
    Ok(FileMetadata {
        relative_path: relative_path(base_path, path)?,
        file_size: 0,
        modified: modified.to_rfc3339_opts(SecondsFormat::Nanos, true),
        kind: FileKind::Directory,
        content_hash: directory_hash(),
        link_target: None,
        unix_mode: unix_mode(path)?,
    })
}

fn capture_symlink(base_path: &Path, path: &Path) -> io::Result<FileMetadata> {
    for _ in 0..2 {
        let before = fs::symlink_metadata(path)?;
        let target = fs::read_link(path)?;
        let target_text = target
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 symlink target"))?
            .to_string();
        let kind = classify_symlink(path);
        validate_symlink_kind(kind, path)?;
        let after = fs::symlink_metadata(path)?;

        if before.modified()? == after.modified()?
            && before.len() == after.len()
            && fs::read_link(path)? == target
        {
            let modified: DateTime<Utc> = after.modified()?.into();
            return Ok(FileMetadata {
                relative_path: relative_path(base_path, path)?,
                file_size: target_text.len() as u64,
                modified: modified.to_rfc3339_opts(SecondsFormat::Nanos, true),
                kind,
                content_hash: hash_bytes(target_text.as_bytes()),
                link_target: Some(target_text),
                unix_mode: None,
            });
        }
    }

    Err(io::Error::other(format!(
        "symlink changed repeatedly while being captured: {}",
        path.display()
    )))
}

#[cfg(unix)]
fn classify_symlink(_path: &Path) -> FileKind {
    FileKind::SymlinkUnknown
}

#[cfg(unix)]
fn validate_symlink_kind(_kind: FileKind, _path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn validate_symlink_kind(kind: FileKind, path: &Path) -> io::Result<()> {
    if kind == FileKind::SymlinkUnknown {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "cannot snapshot a symlink with an unknown target type on Windows: {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn classify_symlink(path: &Path) -> FileKind {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => FileKind::SymlinkDirectory,
        Ok(_) => FileKind::SymlinkFile,
        Err(_) => FileKind::SymlinkUnknown,
    }
}

fn capture_file(
    source_path: &Path,
    destination_path: &Path,
    relative_path: &str,
    previous: Option<&LoadedSnapshotManifest>,
    previous_file: Option<&FileMetadata>,
) -> io::Result<CapturedFile> {
    let Some(previous_file) = previous_file else {
        return copy_file_stable(source_path, destination_path);
    };

    for _ in 0..2 {
        let current = hash_file_stable(source_path)?;
        if current.content_hash == previous_file.content_hash {
            if let Some(previous) = previous {
                let previous_path = join_relative(&previous.data_folder, relative_path)?;
                let previous_is_valid = fs::symlink_metadata(&previous_path)
                    .map(|metadata| metadata.file_type().is_file())
                    .unwrap_or(false)
                    && hash_file_stable(&previous_path)
                        .map(|captured| captured.content_hash == previous_file.content_hash)
                        .unwrap_or(false);

                if previous_is_valid && fs::hard_link(previous_path, destination_path).is_ok() {
                    return Ok(current);
                }
            }
        }

        let copied = copy_file_stable(source_path, destination_path)?;
        if copied.content_hash == current.content_hash {
            return Ok(copied);
        }

        let _ = fs::remove_file(destination_path);
    }

    Err(io::Error::other(format!(
        "file changed repeatedly while being captured: {}",
        source_path.display()
    )))
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
fn protect_snapshot_file(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}

#[cfg(windows)]
fn protect_snapshot_file(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}

#[cfg(not(any(unix, windows)))]
fn protect_snapshot_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn read_ignore_list(base_path: &Path) -> io::Result<Vec<String>> {
    let ignore_path = base_path.join(IGNORE_FILE);
    if !ignore_path.exists() {
        return Ok(Vec::new());
    }

    let reader = io::BufReader::new(fs::File::open(ignore_path)?);
    let mut items = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            items.push(line.to_string());
        }
    }
    Ok(items)
}

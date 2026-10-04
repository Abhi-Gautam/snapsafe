use crate::confirmation::confirm;
use crate::constants::{IGNORE_FILE, REPO_FOLDER, TEMP_FOLDER};
use crate::info;
use crate::integrity::{copy_file_stable, hash_file_stable};
use crate::manifest::{self, load_head_manifest, LoadedSnapshotManifest};
use crate::models::{FileKind, SnapshotIndex};
use crate::paths::{join_relative, relative_path};
use crate::repository::{ensure_layout, recover_transactions, RepositoryLock};
use crate::subcommands::snapshot::{self, SnapshotOptions};
use chrono::Utc;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn restore_snapshot(
    snapshot_id: Option<String>,
    backup: bool,
    dry_run: bool,
    assume_yes: bool,
) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let _lock = RepositoryLock::acquire(&base_path)?;
    ensure_layout(&base_path)?;
    let existing_head = load_head_manifest(&base_path)?;
    recover_transactions(&base_path, &existing_head)?;
    let head_manifest = load_head_manifest(&base_path)?;
    let version = info::resolve_snapshot_id(snapshot_id, &head_manifest)?;
    let snapshot = head_manifest
        .iter()
        .find(|snapshot| snapshot.version == version)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "snapshot not found"))?;
    let loaded = manifest::load_snapshot_manifest(&base_path, &version)?;
    manifest::validate_loaded_manifest(&loaded, &head_manifest, &version)?;

    verify_restore_sources(&loaded)?;
    let plan = build_restore_plan(&base_path, &loaded)?;
    print_restore_plan(snapshot, &plan);

    if dry_run {
        println!("Dry run - no files were changed.");
        return Ok(());
    }

    if !confirm("Continue with restore?", assume_yes)? {
        println!("Restore cancelled.");
        return Ok(());
    }

    let backup_version = if backup {
        println!("Creating backup snapshot before restoring...");
        Some(snapshot::create_snapshot_unlocked(SnapshotOptions {
            message: Some(format!("Auto-backup before restoring {}", version)),
            ignore: Some(loaded.ignore.clone()),
            ..SnapshotOptions::default()
        })?)
    } else {
        None
    };

    let staging_path = base_path.join(REPO_FOLDER).join(TEMP_FOLDER).join(format!(
        "restore-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&staging_path)?;

    let result = stage_restore_files(&loaded, &staging_path)
        .and_then(|_| apply_restore_plan(&base_path, &staging_path, &plan, &loaded));
    let _ = fs::remove_dir_all(&staging_path);

    if let Err(error) = result {
        let recovery = backup_version
            .map(|version| format!("; recovery snapshot: {}", version))
            .unwrap_or_default();
        return Err(io::Error::new(
            error.kind(),
            format!("restore incomplete{}: {}", recovery, error),
        ));
    }

    println!("Snapshot {} restored successfully.", version);
    Ok(())
}

#[derive(Debug)]
struct RestorePlan {
    restore_entries: Vec<String>,
    create_directories: Vec<String>,
    create_count: usize,
    replace_count: usize,
    remove_files: Vec<String>,
    remove_directories: Vec<String>,
}

#[derive(Debug)]
struct WorkingEntry {
    path: String,
    kind: FileKind,
}

fn verify_restore_sources(loaded: &LoadedSnapshotManifest) -> io::Result<()> {
    for file in loaded.files.values() {
        if matches!(
            file.kind,
            FileKind::Directory
                | FileKind::SymlinkFile
                | FileKind::SymlinkDirectory
                | FileKind::SymlinkUnknown
        ) {
            continue;
        }

        let source_path = join_relative(&loaded.data_folder, &file.relative_path)?;
        let captured = hash_file_stable(&source_path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot restore {}: {}", file.relative_path, error),
            )
        })?;

        if captured.content_hash != file.content_hash {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("cannot restore corrupt file {}", file.relative_path),
            ));
        }
    }
    Ok(())
}

fn build_restore_plan(
    base_path: &Path,
    loaded: &LoadedSnapshotManifest,
) -> io::Result<RestorePlan> {
    let expected: std::collections::HashMap<&str, FileKind> = loaded
        .files
        .values()
        .map(|entry| (entry.relative_path.as_str(), entry.kind))
        .collect();
    let mut working = Vec::new();
    collect_managed_entries(base_path, base_path, &loaded.ignore, &mut working)?;

    let mut restore_entries: Vec<String> = loaded
        .files
        .values()
        .filter(|entry| entry.kind != FileKind::Directory)
        .map(|entry| entry.relative_path.clone())
        .collect();
    restore_entries.sort();

    let mut create_directories: Vec<String> = loaded
        .files
        .values()
        .filter(|entry| entry.kind == FileKind::Directory)
        .map(|entry| entry.relative_path.clone())
        .collect();
    create_directories.sort_by_key(|path| path.matches('/').count());

    let working_kinds: std::collections::HashMap<&str, FileKind> = working
        .iter()
        .map(|entry| (entry.path.as_str(), entry.kind))
        .collect();
    let mut create_count = 0;
    let mut replace_count = 0;
    for entry in loaded.files.values() {
        match working_kinds.get(entry.relative_path.as_str()) {
            Some(current_kind)
                if *current_kind != entry.kind || entry.kind != FileKind::Directory =>
            {
                replace_count += 1;
            }
            Some(_) => {}
            None => create_count += 1,
        }
    }

    let mut remove_files: Vec<String> = working
        .iter()
        .filter(|entry| entry.kind != FileKind::Directory)
        .filter(|entry| match expected.get(entry.path.as_str()) {
            Some(FileKind::Directory) | None => true,
            Some(_) => false,
        })
        .map(|entry| entry.path.clone())
        .collect();
    remove_files.sort();

    let mut remove_directories: Vec<String> = working
        .iter()
        .filter(|entry| entry.kind == FileKind::Directory)
        .filter(|entry| expected.get(entry.path.as_str()) != Some(&FileKind::Directory))
        .map(|entry| entry.path.clone())
        .collect();
    remove_directories.sort_by_key(|path| std::cmp::Reverse(path.matches('/').count()));

    Ok(RestorePlan {
        restore_entries,
        create_directories,
        create_count,
        replace_count,
        remove_files,
        remove_directories,
    })
}

fn collect_managed_entries(
    base_path: &Path,
    directory: &Path,
    ignore: &[String],
    entries: &mut Vec<WorkingEntry>,
) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 path"))?;

        if name == REPO_FOLDER
            || name == IGNORE_FILE
            || ignore.iter().any(|ignored| ignored == &name)
        {
            continue;
        }

        let path = entry.path();
        let relative = relative_path(base_path, &path)?;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_managed_entries(base_path, &path, ignore, entries)?;
            entries.push(WorkingEntry {
                path: relative,
                kind: FileKind::Directory,
            });
        } else if file_type.is_file() {
            entries.push(WorkingEntry {
                path: relative,
                kind: FileKind::File,
            });
        } else if file_type.is_symlink() {
            entries.push(WorkingEntry {
                path: relative,
                kind: FileKind::SymlinkUnknown,
            });
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported filesystem entry: {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn stage_restore_files(loaded: &LoadedSnapshotManifest, staging_path: &Path) -> io::Result<()> {
    let mut files: Vec<_> = loaded.files.values().collect();
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    for file in files {
        let destination = join_relative(staging_path, &file.relative_path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        match file.kind {
            FileKind::Directory => {}
            FileKind::File => {
                let source = join_relative(&loaded.data_folder, &file.relative_path)?;
                let captured = copy_file_stable(&source, &destination)?;
                if captured.content_hash != file.content_hash {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "snapshot file changed during restore: {}",
                            file.relative_path
                        ),
                    ));
                }
                apply_unix_mode(&destination, file.unix_mode)?;
            }
            FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown => {
                let target = file.link_target.as_deref().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "missing symlink target")
                })?;
                create_symlink(target, &destination, file.kind)?;
            }
        }
    }
    Ok(())
}

fn apply_restore_plan(
    base_path: &Path,
    staging_path: &Path,
    plan: &RestorePlan,
    loaded: &LoadedSnapshotManifest,
) -> io::Result<()> {
    for relative in &plan.remove_files {
        ensure_safe_parents(base_path, relative)?;
        let path = join_relative(base_path, relative)?;
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            fs::remove_file(path)?;
        }
    }

    for relative in &plan.remove_directories {
        ensure_safe_parents(base_path, relative)?;
        let path = join_relative(base_path, relative)?;
        match fs::read_dir(&path) {
            Ok(mut entries) => {
                if entries.next().is_none() {
                    fs::remove_dir(path)?;
                } else if loaded.files.contains_key(relative) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("cannot replace non-empty preserved directory: {}", relative),
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    for relative in &plan.create_directories {
        ensure_safe_parents(base_path, relative)?;
        fs::create_dir_all(join_relative(base_path, relative)?)?;
    }

    for relative in &plan.restore_entries {
        ensure_safe_parents(base_path, relative)?;
        let staged = join_relative(staging_path, relative)?;
        let target = join_relative(base_path, relative)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        replace_file(&staged, &target)?;
    }

    apply_restored_metadata(base_path, loaded)?;
    Ok(())
}

fn ensure_safe_parents(base_path: &Path, relative: &str) -> io::Result<()> {
    let mut current = PathBuf::from(base_path);
    let parts: Vec<&str> = relative.split('/').collect();

    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("restore path crosses symlink: {}", current.display()),
                ));
            }
            Ok(metadata) if !metadata.file_type().is_dir() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("restore parent is not a directory: {}", current.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
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

fn apply_restored_metadata(base_path: &Path, loaded: &LoadedSnapshotManifest) -> io::Result<()> {
    let mut entries: Vec<_> = loaded.files.values().collect();
    entries.sort_by_key(|entry| {
        let directory_order = if entry.kind == FileKind::Directory {
            1
        } else {
            0
        };
        (
            directory_order,
            std::cmp::Reverse(entry.relative_path.matches('/').count()),
        )
    });

    for entry in entries {
        let path = join_relative(base_path, &entry.relative_path)?;
        if entry.kind == FileKind::File || entry.kind == FileKind::Directory {
            apply_unix_mode(&path, entry.unix_mode)?;
        }
        apply_modified_time(&path, &entry.modified, entry.kind)?;
    }
    Ok(())
}

fn apply_modified_time(path: &Path, timestamp: &str, kind: FileKind) -> io::Result<()> {
    let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let time = filetime::FileTime::from_unix_time(
        timestamp.timestamp(),
        timestamp.timestamp_subsec_nanos(),
    );

    if matches!(
        kind,
        FileKind::SymlinkFile | FileKind::SymlinkDirectory | FileKind::SymlinkUnknown
    ) {
        filetime::set_symlink_file_times(path, time, time)
    } else {
        filetime::set_file_times(path, time, time)
    }
}

#[cfg(unix)]
fn apply_unix_mode(path: &Path, mode: Option<u32>) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn apply_unix_mode(_path: &Path, _mode: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &str, destination: &Path, _kind: FileKind) -> io::Result<()> {
    std::os::unix::fs::symlink(target, destination)
}

#[cfg(windows)]
fn create_symlink(target: &str, destination: &Path, kind: FileKind) -> io::Result<()> {
    match kind {
        FileKind::SymlinkDirectory => std::os::windows::fs::symlink_dir(target, destination),
        FileKind::SymlinkFile => std::os::windows::fs::symlink_file(target, destination),
        FileKind::SymlinkUnknown => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cannot restore a symlink with an unknown target type on Windows",
        )),
        FileKind::File | FileKind::Directory => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "non-symlink entry passed to symlink creation",
        )),
    }
}

fn print_restore_plan(snapshot: &SnapshotIndex, plan: &RestorePlan) {
    println!("Restore snapshot: {}", snapshot.version);
    println!("Created: {}", snapshot.timestamp);
    if let Some(message) = &snapshot.message {
        println!("Message: {}", message);
    }
    println!("Files created: {}", plan.create_count);
    println!("Files replaced: {}", plan.replace_count);
    println!(
        "Entries removed: {}",
        plan.remove_files.len() + plan.remove_directories.len()
    );
}

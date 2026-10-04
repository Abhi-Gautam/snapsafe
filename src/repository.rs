use crate::constants::{REPO_FOLDER, SNAPSHOTS_FOLDER, TEMP_FOLDER};
use crate::models::SnapshotIndex;
use crate::paths::{join_relative, validate_snapshot_id};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

pub struct RepositoryLock {
    file: File,
}

impl RepositoryLock {
    pub fn acquire(base_path: &Path) -> io::Result<Self> {
        let repository_path = base_path.join(REPO_FOLDER);
        ensure_real_directory(&repository_path, "repository")?;

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(repository_path.join("lock"))?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}

pub fn ensure_layout(base_path: &Path) -> io::Result<()> {
    let repository_path = base_path.join(REPO_FOLDER);
    ensure_real_directory(&repository_path, "repository")?;
    ensure_real_directory(
        &repository_path.join(SNAPSHOTS_FOLDER),
        "snapshots directory",
    )?;
    ensure_real_directory(&repository_path.join(TEMP_FOLDER), "temporary directory")?;
    Ok(())
}

pub fn ensure_real_directory(path: &Path, label: &str) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{} not found: {}", label, path.display()),
            )
        } else {
            error
        }
    })?;

    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a real directory: {}", label, path.display()),
        ));
    }
    Ok(())
}

pub fn recover_transactions(base_path: &Path, head: &[SnapshotIndex]) -> io::Result<()> {
    ensure_layout(base_path)?;
    let repository_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repository_path.join(SNAPSHOTS_FOLDER);
    let temporary_path = repository_path.join(TEMP_FOLDER);

    for entry in std::fs::read_dir(&temporary_path)? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 transaction name")
        })?;
        let path = entry.path();

        if name.starts_with("snapshot-") && name.ends_with(".pending") {
            recover_snapshot_transaction(&path, &snapshots_path, head)?;
        } else if name.starts_with("prune-") {
            recover_prune_transaction(&path, &snapshots_path, head)?;
        } else if name.starts_with("snapshot-") || name.starts_with("restore-") {
            remove_tree(&path)?;
        }
    }
    Ok(())
}

fn recover_snapshot_transaction(
    marker_path: &Path,
    snapshots_path: &Path,
    head: &[SnapshotIndex],
) -> io::Result<()> {
    ensure_real_file(marker_path, "snapshot transaction marker")?;
    let content = std::fs::read_to_string(marker_path)?;
    let mut lines = content.lines();
    let version = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid snapshot marker"))?;
    let staging_name = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid snapshot marker"))?;
    validate_snapshot_id(version)?;
    validate_transaction_name(staging_name, "snapshot-")?;

    let final_path = join_relative(snapshots_path, version)?;
    let staging_path = marker_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid marker path"))?
        .join(staging_name);

    if head.iter().any(|snapshot| snapshot.version == version) {
        if !final_path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("committed snapshot directory is missing: {}", version),
            ));
        }
    } else if final_path.exists() {
        remove_tree(&final_path)?;
    }

    if staging_path.exists() {
        remove_tree(&staging_path)?;
    }
    std::fs::remove_file(marker_path)?;
    Ok(())
}

fn recover_prune_transaction(
    trash_path: &Path,
    snapshots_path: &Path,
    head: &[SnapshotIndex],
) -> io::Result<()> {
    ensure_real_directory(trash_path, "prune transaction directory")?;
    for entry in std::fs::read_dir(trash_path)? {
        let entry = entry?;
        let version = entry
            .file_name()
            .into_string()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 snapshot ID"))?;
        validate_snapshot_id(&version)?;
        let temporary = entry.path();

        if head.iter().any(|snapshot| snapshot.version == version) {
            let original = join_relative(snapshots_path, &version)?;
            if original.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("cannot recover snapshot {}; destination exists", version),
                ));
            }
            std::fs::rename(temporary, original)?;
        } else {
            remove_tree(&temporary)?;
        }
    }
    std::fs::remove_dir(trash_path)?;
    Ok(())
}

fn validate_transaction_name(name: &str, prefix: &str) -> io::Result<()> {
    if !name.starts_with(prefix)
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid transaction path",
        ));
    }
    Ok(())
}

pub fn remove_tree(path: &Path) -> io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || metadata.file_type().is_file() {
        return std::fs::remove_file(path);
    }

    #[cfg(windows)]
    make_tree_writable(path)?;
    std::fs::remove_dir_all(path)
}

#[cfg(windows)]
fn make_tree_writable(path: &Path) -> io::Result<()> {
    for entry in walk(path)? {
        let mut permissions = std::fs::symlink_metadata(&entry)?.permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(entry, permissions)?;
    }
    Ok(())
}

#[cfg(windows)]
fn walk(root: &Path) -> io::Result<Vec<std::path::PathBuf>> {
    let mut paths = vec![root.to_path_buf()];
    if std::fs::symlink_metadata(root)?.file_type().is_dir() {
        for entry in std::fs::read_dir(root)? {
            paths.extend(walk(&entry?.path())?);
        }
    }
    paths.reverse();
    Ok(paths)
}

pub fn ensure_real_file(path: &Path, label: &str) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a real file: {}", label, path.display()),
        ));
    }
    Ok(())
}

impl Drop for RepositoryLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

use crate::models::{ContentHash, BLAKE3_ALGORITHM};
use chrono::{DateTime, SecondsFormat, Utc};
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::SystemTime;

const BUFFER_SIZE: usize = 1024 * 1024;
const MAX_ATTEMPTS: usize = 2;

#[derive(Debug, Clone)]
pub struct CapturedFile {
    pub size: u64,
    pub modified: String,
    pub content_hash: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileState {
    size: u64,
    modified: SystemTime,
}

impl FileState {
    fn from_metadata(metadata: &Metadata) -> io::Result<Self> {
        Ok(Self {
            size: metadata.len(),
            modified: metadata.modified()?,
        })
    }
}

pub fn hash_bytes(bytes: &[u8]) -> ContentHash {
    ContentHash::blake3(blake3::hash(bytes).to_hex().to_string())
}

pub fn directory_hash() -> ContentHash {
    hash_bytes(b"snapsafe:directory:v2")
}

pub fn validate_hash(hash: &ContentHash) -> io::Result<()> {
    if hash.algorithm != BLAKE3_ALGORITHM {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported hash algorithm: {}", hash.algorithm),
        ));
    }

    if hash.digest.len() != 64 || !hash.digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid BLAKE3 digest",
        ));
    }

    Ok(())
}

pub fn hash_file_stable(path: &Path) -> io::Result<CapturedFile> {
    for _ in 0..MAX_ATTEMPTS {
        let mut file = File::open(path)?;
        let before = file.metadata()?;
        ensure_regular_file(path, &before)?;
        let before_state = FileState::from_metadata(&before)?;

        let content_hash = hash_reader(&mut file)?;
        let after = file.metadata()?;
        let after_state = FileState::from_metadata(&after)?;

        if before_state == after_state {
            return Ok(CapturedFile {
                size: after_state.size,
                modified: format_system_time(after_state.modified),
                content_hash,
            });
        }
    }

    Err(file_changed_error(path))
}

pub fn copy_file_stable(source_path: &Path, destination_path: &Path) -> io::Result<CapturedFile> {
    for _ in 0..MAX_ATTEMPTS {
        let mut source = File::open(source_path)?;
        let before = source.metadata()?;
        ensure_regular_file(source_path, &before)?;
        let before_state = FileState::from_metadata(&before)?;

        let mut destination = File::create(destination_path)?;
        let content_hash = copy_and_hash(&mut source, &mut destination)?;
        destination.sync_all()?;

        let after = source.metadata()?;
        let after_state = FileState::from_metadata(&after)?;

        if before_state == after_state {
            return Ok(CapturedFile {
                size: after_state.size,
                modified: format_system_time(after_state.modified),
                content_hash,
            });
        }
    }

    let _ = fs::remove_file(destination_path);
    Err(file_changed_error(source_path))
}

fn hash_reader(reader: &mut File) -> io::Result<ContentHash> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(ContentHash::blake3(hasher.finalize().to_hex().to_string()))
}

fn copy_and_hash(source: &mut File, destination: &mut File) -> io::Result<ContentHash> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; BUFFER_SIZE];

    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let bytes = &buffer[..bytes_read];
        hasher.update(bytes);
        destination.write_all(bytes)?;
    }

    Ok(ContentHash::blake3(hasher.finalize().to_hex().to_string()))
}

fn ensure_regular_file(path: &Path, metadata: &Metadata) -> io::Result<()> {
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a regular file: {}", path.display()),
        ));
    }
    Ok(())
}

fn format_system_time(time: SystemTime) -> String {
    let timestamp: DateTime<Utc> = time.into();
    timestamp.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn file_changed_error(path: &Path) -> io::Error {
    io::Error::other(format!(
        "file changed while being captured: {}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, validate_hash};
    use crate::models::ContentHash;

    #[test]
    fn hashes_are_stable_and_content_sensitive() {
        assert_eq!(hash_bytes(b"same"), hash_bytes(b"same"));
        assert_ne!(hash_bytes(b"same"), hash_bytes(b"diff"));
    }

    #[test]
    fn rejects_unknown_algorithms() {
        let hash = ContentHash {
            algorithm: "sha256".to_string(),
            digest: "0".repeat(64),
        };

        assert!(validate_hash(&hash).is_err());
    }
}

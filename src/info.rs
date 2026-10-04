use crate::models::SnapshotIndex;
use crate::paths::validate_snapshot_id;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

pub fn get_base_dir() -> io::Result<PathBuf> {
    std::env::current_dir()
}

pub fn get_next_version(head: &[SnapshotIndex], requested: Option<String>) -> io::Result<String> {
    if let Some(requested) = requested {
        let version = SnapshotVersion::from_str(&requested)?.to_string();
        if head.iter().any(|snapshot| snapshot.version == version) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("snapshot {} already exists", version),
            ));
        }
        return Ok(version);
    }

    if head.is_empty() {
        return Ok(SnapshotVersion([1, 0, 0, 0]).to_string());
    }

    let mut version = SnapshotVersion::from_str(&head.last().unwrap().version)?;
    loop {
        version.0[3] = version.0[3].checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "snapshot version overflow")
        })?;
        let candidate = version.to_string();
        if !head.iter().any(|snapshot| snapshot.version == candidate) {
            return Ok(candidate);
        }
    }
}

pub fn resolve_snapshot_id(
    snapshot_id: Option<String>,
    head_manifest: &[SnapshotIndex],
) -> io::Result<String> {
    if head_manifest.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no snapshots available",
        ));
    }

    let Some(id) = snapshot_id else {
        return Ok(head_manifest.last().unwrap().version.clone());
    };

    if id.eq_ignore_ascii_case("latest") {
        return Ok(head_manifest.last().unwrap().version.clone());
    }

    validate_snapshot_id(&id)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;

    if let Some(snapshot) = head_manifest.iter().find(|snapshot| snapshot.version == id) {
        return Ok(snapshot.version.clone());
    }

    let matches: Vec<&SnapshotIndex> = head_manifest
        .iter()
        .filter(|snapshot| snapshot.version.starts_with(&id))
        .collect();

    match matches.as_slice() {
        [] => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("snapshot {} not found", id),
        )),
        [snapshot] => Ok(snapshot.version.clone()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("snapshot prefix {} is ambiguous", id),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SnapshotVersion([u32; 4]);

impl FromStr for SnapshotVersion {
    type Err = io::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let value = input.strip_prefix('v').unwrap_or(input);
        let parts: Vec<&str> = value.split('.').collect();
        if parts.is_empty() || parts.len() > 4 || parts.iter().any(|part| part.is_empty()) {
            return Err(invalid_version(input));
        }

        let mut version = [0; 4];
        for (index, part) in parts.iter().enumerate() {
            if !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid_version(input));
            }
            version[index] = part.parse().map_err(|_| invalid_version(input))?;
        }

        Ok(Self(version))
    }
}

impl fmt::Display for SnapshotVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "v{}.{}.{}.{}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

fn invalid_version(version: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid snapshot version: {}", version),
    )
}

#[cfg(test)]
mod tests {
    use super::{get_next_version, resolve_snapshot_id, SnapshotVersion};
    use crate::models::{ContentHash, SnapshotIndex};
    use std::str::FromStr;

    fn snapshot(version: &str) -> SnapshotIndex {
        SnapshotIndex {
            version: version.to_string(),
            timestamp: String::new(),
            message: None,
            manifest_hash: ContentHash::blake3("0".repeat(64)),
            metadata: None,
        }
    }

    #[test]
    fn normalizes_versions() {
        assert_eq!(
            SnapshotVersion::from_str("2").unwrap().to_string(),
            "v2.0.0.0"
        );
        assert_eq!(
            SnapshotVersion::from_str("v2.3.4").unwrap().to_string(),
            "v2.3.4.0"
        );
        assert!(SnapshotVersion::from_str("release").is_err());
        assert!(SnapshotVersion::from_str("").is_err());
    }

    #[test]
    fn generates_unused_versions() {
        let head = vec![snapshot("v1.0.0.0"), snapshot("v1.0.0.1")];
        assert_eq!(get_next_version(&head, None).unwrap(), "v1.0.0.2");
        assert!(get_next_version(&head, Some("1".to_string())).is_err());
    }

    #[test]
    fn rejects_ambiguous_prefixes() {
        let head = vec![snapshot("v1.0.0.0"), snapshot("v1.0.0.1")];
        assert!(resolve_snapshot_id(Some("v1".to_string()), &head).is_err());
        assert_eq!(
            resolve_snapshot_id(Some("v1.0.0.1".to_string()), &head).unwrap(),
            "v1.0.0.1"
        );
    }
}

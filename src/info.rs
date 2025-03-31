use crate::models::SnapshotIndex;
use std::io;
use std::path::PathBuf;

/// Returns the base directory (current working directory).
pub fn get_base_dir() -> io::Result<PathBuf> {
    std::env::current_dir()
}

/// Given the current head manifest and an optional user-provided version,
/// returns the next snapshot version string.
pub fn get_next_version(head: &[SnapshotIndex], version: Option<String>) -> String {
    if let Some(user_version) = version {
        // Handle different version input formats
        // If it's already a full version with a "v" prefix, use it directly
        if user_version.starts_with('v') && user_version.matches('.').count() == 3 {
            // Check if this version already exists
            if head.iter().any(|s| s.version == user_version) {
                // Version exists, increment the build number
                let parts: Vec<&str> = user_version.trim_start_matches('v').split('.').collect();
                let major = parts[0];
                let minor = parts[1];
                let patch = parts[2];
                let build: u32 = parts[3].parse().unwrap_or(0);
                let new_build = build + 1;
                format!("v{}.{}.{}.{}", major, minor, patch, new_build)
            } else {
                user_version
            }
        }
        // If it's a simple number like "1" or "2"
        else if user_version.chars().all(|c| c.is_ascii_digit()) {
            format!("v{}.0.0.0", user_version)
        }
        // If it's a partial version like "1.2" or "2.3.1"
        else {
            let trimmed = user_version.trim_start_matches('v');
            let parts: Vec<&str> = trimmed.split('.').collect();

            match parts.len() {
                1 => format!("v{}.0.0.0", parts[0]),
                2 => format!("v{}.{}.0.0", parts[0], parts[1]),
                3 => format!("v{}.{}.{}.0", parts[0], parts[1], parts[2]),
                4 => format!("v{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]),
                _ => "v1.0.0.0".to_string(), // Fallback for unexpected formats
            }
        }
    } else {
        // No version provided, use the auto-incrementing logic
        if head.is_empty() {
            "v1.0.0.0".to_string()
        } else {
            let last_version = &head.last().unwrap().version;
            // Assume the version is in the format vX.Y.Z.B
            let numeric_part = last_version.trim_start_matches('v');
            let parts: Vec<&str> = numeric_part.split('.').collect();
            if parts.len() != 4 {
                // Fallback if not in expected format
                "v1.0.0.0".to_string()
            } else {
                let major = parts[0];
                let minor = parts[1];
                let patch = parts[2];
                let build: u32 = parts[3].parse().unwrap_or(0);
                let new_build = build + 1;
                format!("v{}.{}.{}.{}", major, minor, patch, new_build)
            }
        }
    }
}

/// Resolves a snapshot ID, with support for:
/// - None (returns the latest snapshot)
/// - "latest" (returns the latest snapshot)
/// - Exact version match
/// - Prefix version match
pub fn resolve_snapshot_id(
    snapshot_id: Option<String>,
    head_manifest: &[SnapshotIndex],
) -> io::Result<String> {
    if head_manifest.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No snapshots available.",
        ));
    }

    match snapshot_id {
        None => {
            // If no ID provided, use the latest snapshot
            Ok(head_manifest.last().unwrap().version.clone())
        }
        Some(id) => {
            // Check if the ID is "latest"
            if id.to_lowercase() == "latest" {
                Ok(head_manifest.last().unwrap().version.clone())
            } else {
                // Try exact match first
                let exact_match = head_manifest
                    .iter()
                    .find(|s| s.version == id)
                    .map(|s| s.version.clone());

                // If no exact match, try prefix match
                match exact_match {
                    Some(v) => Ok(v),
                    None => head_manifest
                        .iter()
                        .find(|s| s.version.starts_with(&id))
                        .map(|s| s.version.clone())
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::NotFound,
                                format!("Snapshot {} not found", id),
                            )
                        }),
                }
            }
        }
    }
}

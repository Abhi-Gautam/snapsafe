use crate::models::SnapshotIndex;
use std::io;
use std::path::PathBuf;

/// Returns the base directory (current working directory).
pub fn get_base_dir() -> io::Result<PathBuf> {
    std::env::current_dir()
}

/// Given the current head manifest and an optional user-provided tag,
/// returns the next snapshot version string.
/// - If a tag is provided, uses that (prefixed with "v" if needed).
/// - Otherwise, if no snapshots exist, returns "v1.0.0.0".
/// - Otherwise, increments the build number (the last component) of the last snapshot version.
pub fn get_next_version(head: &Vec<SnapshotIndex>, tag: Option<String>) -> String {
    if let Some(user_tag) = tag {
        if user_tag.starts_with('v') {
            user_tag
        } else {
            format!("v{}", user_tag)
        }
    } else {
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
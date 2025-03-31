use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Structure to hold metadata for a single file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadata {
    /// The file's path relative to the base directory.
    pub relative_path: String,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modification time as a formatted string.
    pub modified: String,
}

/// Structure for custom metadata attached to a snapshot
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SnapshotMetadata {
    /// Tags assigned to the snapshot
    pub tags: Vec<String>,
    /// Custom key-value properties
    pub custom: HashMap<String, String>,
}

/// Structure to represent a snapshot entry in the head manifest.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SnapshotIndex {
    /// The version string (e.g., "v1.0.0.0" or "vrelease" if provided).
    pub version: String,
    /// The snapshot creation timestamp (as a string).
    pub timestamp: String,
    /// An optional message provided by the user.
    pub message: Option<String>,
    /// Optional metadata for the snapshot
    #[serde(default)]
    pub metadata: Option<SnapshotMetadata>,
}
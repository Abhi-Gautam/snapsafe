use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const FORMAT_VERSION: u32 = 2;
pub const BLAKE3_ALGORITHM: &str = "blake3";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ContentHash {
    pub algorithm: String,
    pub digest: String,
}

impl ContentHash {
    pub fn blake3(digest: impl Into<String>) -> Self {
        Self {
            algorithm: BLAKE3_ALGORITHM.to_string(),
            digest: digest.into(),
        }
    }

    pub fn is_supported(&self) -> bool {
        self.algorithm == BLAKE3_ALGORITHM
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    File,
    Directory,
    SymlinkFile,
    SymlinkDirectory,
    SymlinkUnknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub relative_path: String,
    pub file_size: u64,
    pub modified: String,
    pub kind: FileKind,
    pub content_hash: ContentHash,
    pub link_target: Option<String>,
    pub unix_mode: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SnapshotManifest {
    pub format_version: u32,
    pub ignore: Vec<String>,
    pub files: Vec<FileMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub tags: Vec<String>,
    pub custom: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SnapshotIndex {
    pub version: String,
    pub timestamp: String,
    pub message: Option<String>,
    pub manifest_hash: ContentHash,
    pub metadata: Option<SnapshotMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HeadManifest {
    pub format_version: u32,
    pub snapshots: Vec<SnapshotIndex>,
}

pub const REPO_FOLDER: &str = ".snapsafe";
pub const SNAPSHOTS_FOLDER: &str = "snapshots";
pub const HEAD_MANIFEST_FILE: &str = "head_manifest.json";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const IGNORE_FILE: &str = ".snapsafeignore";

pub const DEFAULT_IGNORE_ITEMS: &[&str] = &[
    ".git",
    ".gitignore",
    "target",
    ".DS_Store",
    ".snapsafeignore",
];
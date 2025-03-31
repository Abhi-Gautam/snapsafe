use clap::{Parser, Subcommand};
use std::process;
mod models;
mod info;
mod manifest;
mod constants;
mod subcommands;

/// Snap Safe: A CLI tool for efficient directory snapshots management
///
/// Snap Safe leverages hard links to create space-efficient, incremental
/// snapshots of your directory contents. It's designed to manage file versioning
/// with minimal overhead, perfect for build artifacts, large binary files,
/// configuration management, and deployment tracking.
#[derive(Parser)]
#[command(name = "snapsafe")]
#[command(about = "Snap Safe: A CLI tool for efficient snapshots management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes a new Snap Safe repository in the current directory
    ///
    /// This command creates a hidden .snapsafe directory structure to store 
    /// snapshots and metadata. It's the first command you should run before
    /// using other Snap Safe features.
    ///
    /// Example: snapsafe init
    Init,

    /// Create a new snapshot of the current directory state
    ///
    /// Creates a space-efficient snapshot by hard-linking unchanged files from previous
    /// snapshots and only copying modified files. Snapshots can be annotated with 
    /// messages, tags, and custom metadata.
    ///
    /// Examples:
    ///   snapsafe snapshot -m "Initial snapshot"
    ///   snapsafe snapshot -v "2.0.0.0" -m "Release candidate"
    ///   snapsafe snapshot --tags production release --meta ran_by SCM
    Snapshot {
        /// Optional custom version for the snapshot (e.g., "v1.2.3.4", "2", "3.0", etc.)
        /// If not provided, the version will auto-increment from the last snapshot
        #[arg(short, long)]
        version: Option<String>,
        /// Optional message describing the snapshot
        #[arg(short, long)]
        message: Option<String>,
        /// Add tags to the snapshot
        #[arg(long, num_args = 1..)]
        tags: Option<Vec<String>>,
        /// Add custom metadata to the snapshot (key and value pair)
        /// This can store arbitrary information like build IDs, environment details, etc.
        #[arg(long, num_args = 2, value_names = &["KEY", "VALUE"])]
        meta: Option<Vec<String>>,
    },
    /// List all snapshots
    List,
    /// Show differences between two snapshots
    ///
    /// Compares two snapshots and displays files that were added, removed,
    /// or modified between them. If only one snapshot is specified, it's
    /// compared with the latest snapshot.
    ///
    /// Examples:
    ///   snapsafe diff v1.0.0.0 v1.0.0.1
    ///   snapsafe diff v1.0.0.0  # Compares with latest snapshot
    Diff {
        /// First snapshot ID
        snapshot1: String,
        /// Optional Second snapshot ID
        /// If not provided, defaults to the latest snapshot
        snapshot2: Option<String>,
    },
    /// Restore the working directory to a snapshot state
    ///
    /// Restores all files from a snapshot to the working directory,
    /// effectively reverting to that point in time. By default, it creates
    /// a backup snapshot before restoring.
    ///
    /// Examples:
    ///   snapsafe restore v1.0.0.0
    ///   snapsafe restore latest
    ///   snapsafe restore v1.0.0.0 --no-backup
    Restore {
        /// Snapshot ID to restore (version, prefix, or "latest")
        /// If not provided, restores the latest snapshot
        snapshot_id: Option<String>,
        
        /// Skip creating a backup snapshot before restoring
        /// Note: Without a backup, you can't easily undo the restoration
        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_backup: bool,
    },
    /// Remove old snapshots based on specified criteria
    ///
    /// Helps manage disk space by removing snapshots that are no longer needed.
    /// Can prune based on count (keeping N most recent) or age (removing older
    /// than a specified duration).
    ///
    /// Examples:
    ///   snapsafe prune --keep-last 5
    ///   snapsafe prune --older-than 7d
    ///   snapsafe prune --older-than 30d --dry-run
    Prune {
        /// Keep only the N most recent snapshots and remove older ones
        #[arg(long)]
        keep_last: Option<usize>,
        
        /// Remove snapshots older than the specified duration
        /// Supports formats: "7d" (days), "24h" (hours), "30m" (minutes), "60s" (seconds)
        #[arg(long)]
        older_than: Option<String>,
        
        /// Simulate pruning without actually deleting snapshots
        /// Shows what would be removed without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Verify the integrity of snapshots
    ///
    /// Scans snapshots to ensure all files are present and uncorrupted.
    /// Useful for confirming snapshot validity, especially after moving
    /// or copying the repository.
    ///
    /// Examples:
    ///   snapsafe verify
    ///   snapsafe verify v1.0.0.0
    Verify {
        /// Verify only the specified snapshot ID 
        /// If not provided, verifies all snapshots
        snapshot_id: Option<String>,
    },
    /// Show detailed information about a snapshot
    ///
    /// Displays comprehensive details about a snapshot, including statistics
    /// like file count, total size, file types, largest files, etc.
    ///
    /// Examples:
    ///   snapsafe info v1.0.0.0
    ///   snapsafe info 
    Info {
        /// Snapshot ID to show information
        /// If not provided, shows information for the latest snapshot
        snapshot_id: Option<String>,
    },
    /// Configure Snap Safe settings
    ///
    /// Manages Snap Safe configuration options. Settings control behavior
    /// like automatic backups, compression, and default snapshot messages.
    ///
    /// Examples:
    ///   snapsafe config --set autobackup false
    ///   snapsafe config --get autobackup
    ///   snapsafe config --list
    Config {
        /// Set a configuration key and value
        /// Available keys: autobackup, compression, default_snapshot_message,
        /// max_backups, verify_after_snapshot, text_diff_extensions
        #[arg(short, long, num_args = 2)]
        set: Option<Vec<String>>,
        
        /// Get the value of a configuration key
        #[arg(short, long)]
        get: Option<String>,
        
        /// List all configuration settings and their current values
        #[arg(short, long)]
        list: bool,
    },
    /// Manage tags for snapshots
    ///
    /// Adds, removes, or lists tags associated with snapshots.
    /// Tags help categorize and organize snapshots for easier management.
    ///
    /// Examples:
    ///   snapsafe tag v1.0.0.0 --add production stable
    ///   snapsafe tag v1.0.0.0 --remove unstable
    ///   snapsafe tag v1.0.0.0 --list
    Tag {
        /// Snapshot ID to manage tags
        /// If not provided, defaults to the latest snapshot
        snapshot_id: Option<String>,
        
        /// Add one or more tags to the snapshot
        #[arg(short, long, num_args = 1..)]
        add: Option<Vec<String>>,
        
        /// Remove one or more tags from the snapshot
        #[arg(short, long, num_args = 1..)]
        remove: Option<Vec<String>>,
        
        /// List all tags for the snapshot (default if no other options provided)
        #[arg(short, long)]
        list: bool,
    },
    
    /// Manage custom metadata for snapshots
    ///
    /// Sets, removes, or lists custom key-value metadata for snapshots.
    /// Metadata lets you store arbitrary information with your snapshots.
    ///
    /// Examples:
    ///   snapsafe meta v1.0.0.0 --set build_id 12345
    ///   snapsafe meta v1.0.0.0 --remove build_id
    ///   snapsafe meta v1.0.0.0 --list
    Meta {
        /// Snapshot ID to manage metadata
        /// If not provided, defaults to the latest snapshot
        snapshot_id: Option<String>,
        
        /// Set a metadata key and value
        #[arg(short, long, num_args = 2)]
        set: Option<Vec<String>>,
        
        /// Remove a metadata key and its associated value
        #[arg(short, long)]
        remove: Option<String>,
        
        /// List all metadata for the snapshot (default if no other options provided)
        #[arg(short, long)]
        list: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            if let Err(e) = subcommands::init::init_repository() {
                eprintln!("Error initializing repository: {}", e);
                process::exit(1);
            }
        },
        Commands::Snapshot { version, message, tags, meta } => {
            // Create the snapshot first
            if let Err(e) = subcommands::snapshot::create_snapshot(message.clone(), version.clone()) {
                eprintln!("Error creating snapshot: {}", e);
                process::exit(1);
            }
            
            // Get the created snapshot version (likely the latest one)
            let base_path = info::get_base_dir().unwrap();
            let head_manifest = manifest::load_head_manifest(&base_path).unwrap();
            if let Some(last_snapshot) = head_manifest.last() {
                let snapshot_id = last_snapshot.version.clone();
                
                // Add tags if provided
                if let Some(tag_list) = tags {
                    if let Err(e) = subcommands::tag::manage_tags(Some(snapshot_id.clone()), Some(tag_list.to_vec()), None, false) {
                        eprintln!("Error adding tags: {}", e);
                    }
                }
                
                // Add metadata if provided
                if let Some(metadata) = meta {
                    if metadata.len() == 2 {
                        if let Err(e) = subcommands::meta::manage_metadata(Some(snapshot_id.clone()), Some(metadata.to_vec()), None, false) {
                            eprintln!("Error adding metadata: {}", e);
                        }
                    } else {
                        eprintln!("Error: Please provide exactly two values for --meta: a key and a value.");
                    }
                }
            }
        },
        Commands::List => {
            if let Err(e) = subcommands::list::list_snapshots() {
                eprintln!("Error listing snapshots: {}", e);
                process::exit(1);
            }
        },
        Commands::Diff { snapshot1, snapshot2 } => {
            if let Err(e) = subcommands::diff::diff_snapshots(snapshot1.clone(), snapshot2.clone()) {
                eprintln!("Error diffing snapshots: {}", e);
                process::exit(1);
            }
        },
        Commands::Restore { snapshot_id, no_backup } => {
            let backup = !no_backup; // Invert the flag since we want backup by default
            if let Err(e) = subcommands::restore::restore_snapshot(snapshot_id.clone(), backup) {
                eprintln!("Error restoring snapshot: {}", e);
                process::exit(1);
            }
        },
        Commands::Prune { keep_last, older_than, dry_run } => {
            if let Err(e) = subcommands::prune::prune_snapshots(*keep_last, older_than.clone(), *dry_run) {
                eprintln!("Error pruning snapshots: {}", e);
                process::exit(1);
            }
        },
        Commands::Verify { snapshot_id } => {
            if let Err(e) = subcommands::verify::verify_snapshots(snapshot_id.clone()) {
                eprintln!("Error verifying snapshots: {}", e);
                process::exit(1);
            }
        },
        Commands::Info { snapshot_id } => {
            if let Err(e) = subcommands::info::show_snapshot_info(snapshot_id.clone()) {
                eprintln!("Error showing snapshot info: {}", e);
                process::exit(1);
            }
        },
        Commands::Config { set, get, list } => {
            if let Err(e) = subcommands::config::configure(set.clone(), get.clone(), *list) {
                eprintln!("Error configuring: {}", e);
                process::exit(1);
            }
        },
        Commands::Tag { snapshot_id, add, remove, list } => {
            if let Err(e) = subcommands::tag::manage_tags(snapshot_id.clone(), add.clone(), remove.clone(), *list) {
                eprintln!("Error managing tags: {}", e);
                process::exit(1);
            }
        },
        Commands::Meta { snapshot_id, set, remove, list } => {
            if let Err(e) = subcommands::meta::manage_metadata(snapshot_id.clone(), set.clone(), remove.clone(), *list) {
                eprintln!("Error managing metadata: {}", e);
                process::exit(1);
            }
        },
    }
}
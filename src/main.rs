use clap::{Parser, Subcommand};
use std::process;
mod models;
mod info;
mod manifest;
mod constants;
mod subcommands;

/// Snap Safe: A CLI tool for efficient snapshots
#[derive(Parser)]
#[command(name = "snapsafe")]
#[command(about = "Snap Safe: A CLI tool for efficient snapshots management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes Snap Safe in the specifier directory.
    Init,

    /// Create a new snapshot
    Snapshot {
        /// Optional version for the snapshot
        #[arg(short, long)]
        version: Option<String>,
        /// Optional message describing the snapshot
        #[arg(short, long)]
        message: Option<String>,
        /// Add tags to the snapshot
        #[arg(long, num_args = 1..)]
        tags: Option<Vec<String>>,
        /// Add metadata to the snapshot in key=value format
        #[arg(long, num_args = 2, value_names = &["KEY", "VALUE"])]
        meta: Option<Vec<String>>,
    },
    /// List all snapshots
    List,
    /// Show differences between two snapshots
    Diff {
        /// First snapshot ID
        snapshot1: String,
        /// Second snapshot ID
        snapshot2: Option<String>,
    },
    /// Restore a snapshot
    Restore {
        /// Snapshot ID to restore
        snapshot_id: Option<String>,
        /// Skip creating a backup before restoring
        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_backup: bool,
    },
    /// Remove old snapshots based on criteria
    Prune {
        /// Keep only the N most recent snapshots
        #[arg(long)]
        keep_last: Option<usize>,
        /// Remove snapshots older than the specified duration (e.g., "7d", "24h", "30m")
        #[arg(long)]
        older_than: Option<String>,
        /// Perform a dry run without actually deleting snapshots
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify the integrity of snapshots
    Verify {
        /// Verify only the specified snapshot (all snapshots if not specified)
        snapshot_id: Option<String>,
    },
    /// Show detailed information about a snapshot
    Info {
        /// Snapshot ID to show information for
        snapshot_id: Option<String>,
    },
    /// Configure Snap Safe settings
    Config {
        /// Set configuration key and value
        #[arg(short, long, num_args = 2)]
        set: Option<Vec<String>>,
        /// Get configuration value for a key
        #[arg(short, long)]
        get: Option<String>,
        /// List all configuration settings
        #[arg(short, long)]
        list: bool,
    },
    /// Manage tags for a snapshot
    Tag {
        /// Snapshot ID to add tags to
        snapshot_id: Option<String>,
        /// Add tags to the snapshot
        #[arg(short, long, num_args = 1..)]
        add: Option<Vec<String>>,
        /// Remove tags from the snapshot
        #[arg(short, long, num_args = 1..)]
        remove: Option<Vec<String>>,
        /// List tags for the snapshot (default if no other options provided)
        #[arg(short, long)]
        list: bool,
    },
    /// Manage custom metadata for a snapshot
    Meta {
        /// Snapshot ID to manage metadata for
        snapshot_id: Option<String>,
        /// Set a metadata key and value
        #[arg(short, long, num_args = 2)]
        set: Option<Vec<String>>,
        /// Remove a metadata key
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
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
        /// Optional tag for the snapshot.
        #[arg(short, long)]
        tag: Option<String>,
        /// Optional message describing the snapshot.
        #[arg(short, long)]
        message: Option<String>,
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
        snapshot_id: String,
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
        snapshot_id: String,
        /// Add tags to the snapshot
        #[arg(short, long)]
        add: Option<Vec<String>>,
        /// Remove tags from the snapshot
        #[arg(short, long)]
        remove: Option<Vec<String>>,
        /// List tags for the snapshot (default if no other options provided)
        #[arg(short, long)]
        list: bool,
    },
    /// Manage custom metadata for a snapshot
    Meta {
        /// Snapshot ID to manage metadata for
        snapshot_id: String,
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
        Commands::Snapshot { tag, message } => {
            if let Err(e) = subcommands::snapshot::create_snapshot(message.clone(), tag.clone()) {
                eprintln!("Error creating snapshot: {}", e);
                process::exit(1);
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
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
    }
}
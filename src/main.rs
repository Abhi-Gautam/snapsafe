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
        no_backup: bool,
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
    }
}
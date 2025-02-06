use std::io;

use crate::{info::get_base_dir, manifest::load_head_manifest};

/// Lists all snapshots by reading the head manifest and printing each entry.
pub fn list_snapshots() -> io::Result<()> {
    let base_path = get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;
    if head_manifest.is_empty() {
        println!("No snapshots found.");
    } else {
        println!("{:<10} {:<20} {:<30}", "Version", "Timestamp", "Message");
        println!("{:-<10} {:-<20} {:-<30}", "", "", "");
        for snapshot in head_manifest {
            let msg = snapshot.message.unwrap_or_default();
            println!("{:<10} {:<20} {:<30}", snapshot.version, snapshot.timestamp, msg);
        }
    }
    Ok(())
}
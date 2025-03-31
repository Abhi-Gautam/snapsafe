use std::io;

use crate::{info::get_base_dir, manifest::load_head_manifest};

/// Lists all snapshots by reading the head manifest and printing each entry.
pub fn list_snapshots() -> io::Result<()> {
    let base_path = get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;
    if head_manifest.is_empty() {
        println!("No snapshots found.");
    } else {
        println!(
            "{:<10} {:<20} {:<20} {:<20} {:<30}",
            "Version", "Timestamp", "Message", "Tags", "Metadata"
        );
        println!(
            "{:-<10} {:-<20} {:-<20} {:-<20} {:-<30}",
            "", "", "", "", ""
        );
        for snapshot in head_manifest {
            let msg = snapshot.message.unwrap_or_default();

            // Format tags as a comma-separated list
            let tags = if let Some(ref metadata) = snapshot.metadata {
                if metadata.tags.is_empty() {
                    "-".to_string()
                } else {
                    metadata.tags.join(", ")
                }
            } else {
                "-".to_string()
            };

            // Format metadata as key=value pairs
            let meta_str = if let Some(ref metadata) = snapshot.metadata {
                if metadata.custom.is_empty() {
                    "-".to_string()
                } else {
                    metadata
                        .custom
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<String>>()
                        .join(", ")
                }
            } else {
                "-".to_string()
            };

            println!(
                "{:<10} {:<20} {:<20} {:<20} {:<30}",
                snapshot.version,
                snapshot.timestamp,
                if msg.len() > 17 {
                    format!("{}...", &msg[..17])
                } else {
                    msg
                },
                if tags.len() > 17 {
                    format!("{}...", &tags[..17])
                } else {
                    tags
                },
                if meta_str.len() > 27 {
                    format!("{}...", &meta_str[..27])
                } else {
                    meta_str
                }
            );
        }
    }
    Ok(())
}

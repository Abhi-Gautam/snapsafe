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
                    let mut entries: Vec<String> = metadata
                        .custom
                        .iter()
                        .map(|(key, value)| format!("{}={}", key, value))
                        .collect();
                    entries.sort();
                    entries.join(", ")
                }
            } else {
                "-".to_string()
            };

            println!(
                "{:<10} {:<20} {:<20} {:<20} {:<30}",
                snapshot.version,
                snapshot.timestamp,
                truncate(&msg, 17),
                truncate(&tags, 17),
                truncate(&meta_str, 27)
            );
        }
    }
    Ok(())
}

fn truncate(value: &str, maximum_characters: usize) -> String {
    let mut characters = value.chars();
    let prefix: String = characters.by_ref().take(maximum_characters).collect();
    if characters.next().is_some() {
        format!("{}...", prefix)
    } else {
        prefix
    }
}

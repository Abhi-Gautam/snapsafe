use std::fs;
use std::io;
use chrono::{NaiveDateTime, Local, TimeZone, Duration};

use crate::constants::{REPO_FOLDER, SNAPSHOTS_FOLDER};
use crate::info;
use crate::manifest::{load_head_manifest, save_head_manifest};

/// Prune snapshots based on age or count
pub fn prune_snapshots(
    keep_last: Option<usize>,
    older_than: Option<String>,
    dry_run: bool,
) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let mut head_manifest = load_head_manifest(&base_path)?;
    
    if head_manifest.is_empty() {
        println!("No snapshots to prune.");
        return Ok(());
    }
    
    // Sort snapshots by timestamp (oldest first)
    head_manifest.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    // Create a list of snapshots to delete
    let mut to_delete = Vec::new();
    
    // If keep_last is specified, keep the N most recent snapshots
    if let Some(keep) = keep_last {
        if keep >= head_manifest.len() {
            println!("Keeping all {} snapshots.", head_manifest.len());
            return Ok(());
        }
        
        let to_keep = head_manifest.len() - keep;
        to_delete.extend(head_manifest.iter().take(to_keep).cloned());
        
        println!("Will keep {} most recent snapshots.", keep);
    }
    
    // If older_than is specified, delete snapshots older than the specified duration
    if let Some(ref duration_str) = older_than {
        let duration = parse_duration(duration_str)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        
        let cutoff_time = Local::now() - duration;
        let cutoff_str = cutoff_time.format("%Y-%m-%d %H:%M:%S").to_string();
        
        println!("Will delete snapshots older than {}", cutoff_str);
        
        for snapshot in &head_manifest {
            // Parse the snapshot timestamp
            if let Ok(snapshot_time) = NaiveDateTime::parse_from_str(&snapshot.timestamp, "%Y-%m-%d %H:%M:%S") {
                if let Some(datetime) = Local.from_local_datetime(&snapshot_time).earliest() {
                    if datetime < cutoff_time && !to_delete.contains(snapshot) {
                        to_delete.push(snapshot.clone());
                    }
                }
            }
        }
    }
    
    // If neither option is specified, do nothing
    if keep_last.is_none() && older_than.is_none() {
        println!("No pruning criteria specified. Use --keep-last or --older-than.");
        return Ok(());
    }
    
    if to_delete.is_empty() {
        println!("No snapshots to prune based on the specified criteria.");
        return Ok(());
    }
    
    // Print the snapshots that will be deleted
    println!("The following snapshots will be {}:", if dry_run { "pruned (dry run)" } else { "pruned" });
    for snapshot in &to_delete {
        println!("  - {} ({})", snapshot.version, snapshot.timestamp);
    }
    
    if dry_run {
        println!("Dry run - no snapshots were deleted.");
        return Ok(());
    }
    
    // Confirm deletion
    println!("Are you sure you want to delete these snapshots? (y/n)");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Pruning cancelled.");
        return Ok(());
    }
    
    // Delete the snapshots
    for snapshot in &to_delete {
        let snapshot_dir = base_path
            .join(REPO_FOLDER)
            .join(SNAPSHOTS_FOLDER)
            .join(&snapshot.version);
        
        if snapshot_dir.exists() {
            fs::remove_dir_all(&snapshot_dir)?;
            println!("Deleted snapshot: {}", snapshot.version);
        }
    }
    
    // Update the head manifest to remove the deleted snapshots
    head_manifest.retain(|s| !to_delete.contains(s));
    save_head_manifest(&base_path, &head_manifest)?;
    
    println!("Pruned {} snapshots.", to_delete.len());
    Ok(())
}

/// Parse a duration string into a chrono::Duration
/// Supports formats like "7d", "24h", "30m"
fn parse_duration(duration_str: &str) -> Result<Duration, String> {
    let mut chars = duration_str.chars();
    let mut num_str = String::new();
    
    // Extract the numeric part
    for c in chars.by_ref() {
        if c.is_ascii_digit() {
            num_str.push(c);
        } else {
            break;
        }
    }
    
    // Extract the unit part
    let unit: String = chars.collect();
    let value: i64 = num_str.parse().map_err(|_| format!("Invalid duration: {}", duration_str))?;
    
    match unit.as_str() {
        "d" | "days" | "day" => Ok(Duration::days(value)),
        "h" | "hours" | "hour" => Ok(Duration::hours(value)),
        "m" | "minutes" | "min" => Ok(Duration::minutes(value)),
        "s" | "seconds" | "sec" => Ok(Duration::seconds(value)),
        _ => Err(format!("Unsupported duration unit: {}. Use d, h, m, or s.", unit)),
    }
}
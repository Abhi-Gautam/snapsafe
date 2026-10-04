use crate::confirmation::confirm;
use crate::constants::{REPO_FOLDER, SNAPSHOTS_FOLDER, TEMP_FOLDER};
use crate::info;
use crate::manifest::{load_head_manifest, save_head_manifest};
use crate::paths::join_relative;
use crate::repository::{
    ensure_layout, ensure_real_directory, recover_transactions, remove_tree, RepositoryLock,
};
use chrono::{DateTime, Duration, Utc};
use std::fs;
use std::io;

pub fn prune_snapshots(
    keep_last: Option<usize>,
    older_than: Option<String>,
    dry_run: bool,
    assume_yes: bool,
) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let _lock = RepositoryLock::acquire(&base_path)?;
    ensure_layout(&base_path)?;
    let existing_head = load_head_manifest(&base_path)?;
    recover_transactions(&base_path, &existing_head)?;
    let mut head_manifest = load_head_manifest(&base_path)?;

    if head_manifest.is_empty() {
        println!("No snapshots to prune.");
        return Ok(());
    }

    if keep_last.is_none() && older_than.is_none() {
        println!("No pruning criteria specified. Use --keep-last or --older-than.");
        return Ok(());
    }

    head_manifest.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    let mut to_delete = Vec::new();

    if let Some(keep) = keep_last {
        let delete_count = head_manifest.len().saturating_sub(keep);
        to_delete.extend(head_manifest.iter().take(delete_count).cloned());
    }

    if let Some(duration) = older_than {
        let cutoff = Utc::now() - parse_duration(&duration)?;
        for snapshot in &head_manifest {
            let timestamp = DateTime::parse_from_rfc3339(&snapshot.timestamp)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
                .with_timezone(&Utc);
            if timestamp < cutoff {
                to_delete.push(snapshot.clone());
            }
        }
    }

    if to_delete.is_empty() {
        println!("No snapshots match the pruning criteria.");
        return Ok(());
    }

    println!("Snapshots to prune:");
    for snapshot in &to_delete {
        println!("  {} ({})", snapshot.version, snapshot.timestamp);
    }

    if dry_run {
        println!("Dry run - no snapshots were deleted.");
        return Ok(());
    }

    if !confirm("Delete these snapshots?", assume_yes)? {
        println!("Pruning cancelled.");
        return Ok(());
    }

    let repository_path = base_path.join(REPO_FOLDER);
    let snapshots_path = repository_path.join(SNAPSHOTS_FOLDER);
    let trash_path = repository_path.join(TEMP_FOLDER).join(format!(
        "prune-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&trash_path)?;

    for snapshot in &to_delete {
        let source = join_relative(&snapshots_path, &snapshot.version)?;
        ensure_real_directory(&source, "snapshot directory")?;
    }

    let mut moved = Vec::new();
    for snapshot in &to_delete {
        let source = join_relative(&snapshots_path, &snapshot.version)?;
        let destination = join_relative(&trash_path, &snapshot.version)?;
        if let Err(error) = fs::rename(&source, &destination) {
            if rollback_moves(&moved).is_ok() {
                let _ = remove_tree(&trash_path);
            }
            return Err(error);
        }
        moved.push((destination, source));
    }

    head_manifest.retain(|snapshot| !to_delete.contains(snapshot));
    if let Err(error) = save_head_manifest(&base_path, &head_manifest) {
        if rollback_moves(&moved).is_ok() {
            let _ = remove_tree(&trash_path);
        }
        return Err(error);
    }

    remove_tree(&trash_path)?;
    println!("Pruned {} snapshot(s).", to_delete.len());
    Ok(())
}

fn rollback_moves(moves: &[(std::path::PathBuf, std::path::PathBuf)]) -> io::Result<()> {
    let mut first_error = None;
    for (temporary, original) in moves.iter().rev() {
        if let Err(error) = fs::rename(temporary, original) {
            first_error.get_or_insert(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn parse_duration(input: &str) -> io::Result<Duration> {
    let unit_start = input
        .find(|character: char| !character.is_ascii_digit())
        .ok_or_else(|| invalid_duration(input))?;
    let (number, unit) = input.split_at(unit_start);
    if number.is_empty() || unit.is_empty() {
        return Err(invalid_duration(input));
    }

    let value: i64 = number.parse().map_err(|_| invalid_duration(input))?;
    let duration = match unit {
        "d" | "day" | "days" => Duration::try_days(value),
        "h" | "hour" | "hours" => Duration::try_hours(value),
        "m" | "min" | "minute" | "minutes" => Duration::try_minutes(value),
        "s" | "sec" | "second" | "seconds" => Duration::try_seconds(value),
        _ => None,
    };

    duration.ok_or_else(|| invalid_duration(input))
}

fn invalid_duration(input: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid duration {}; use d, h, m, or s", input),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_duration;
    use chrono::Duration;

    #[test]
    fn parses_supported_durations() {
        assert_eq!(parse_duration("7d").unwrap(), Duration::days(7));
        assert_eq!(parse_duration("24h").unwrap(), Duration::hours(24));
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("60s").unwrap(), Duration::seconds(60));
    }

    #[test]
    fn rejects_invalid_durations() {
        for value in ["", "7", "d", "-1d", "7weeks"] {
            assert!(parse_duration(value).is_err(), "accepted {value}");
        }
    }
}

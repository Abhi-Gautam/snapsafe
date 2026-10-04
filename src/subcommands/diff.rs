use crate::info;
use crate::manifest::{self, load_head_manifest};
use std::io;

pub fn diff_snapshots(snapshot1: String, snapshot2: Option<String>) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let head_manifest = load_head_manifest(&base_path)?;
    let version1 = info::resolve_snapshot_id(Some(snapshot1), &head_manifest)?;
    let version2 = info::resolve_snapshot_id(snapshot2, &head_manifest)?;

    let manifest1 = manifest::load_snapshot_manifest(&base_path, &version1)?;
    let manifest2 = manifest::load_snapshot_manifest(&base_path, &version2)?;
    manifest::validate_loaded_manifest(&manifest1, &head_manifest, &version1)?;
    manifest::validate_loaded_manifest(&manifest2, &head_manifest, &version2)?;

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut updated = Vec::new();

    for (path, second) in &manifest2.files {
        match manifest1.files.get(path) {
            Some(first)
                if first.kind != second.kind
                    || first.content_hash != second.content_hash
                    || first.link_target != second.link_target
                    || first.unix_mode != second.unix_mode
                    || first.modified != second.modified =>
            {
                updated.push(path.clone())
            }
            None => added.push(path.clone()),
            _ => {}
        }
    }

    for path in manifest1.files.keys() {
        if !manifest2.files.contains_key(path) {
            removed.push(path.clone());
        }
    }

    added.sort();
    removed.sort();
    updated.sort();

    print_section("Added Files", &added);
    print_section("Removed Files", &removed);
    print_section("Updated Files", &updated);

    if added.is_empty() && removed.is_empty() && updated.is_empty() {
        println!(
            "No differences found between snapshots {} and {}.",
            version1, version2
        );
    }

    Ok(())
}

fn print_section(title: &str, files: &[String]) {
    if files.is_empty() {
        return;
    }

    println!("{}:", title);
    println!("{:-<50}", "");
    for file in files {
        println!("{}", file);
    }
    println!();
}

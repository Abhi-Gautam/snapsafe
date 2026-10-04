use assert_cmd::Command;
use filetime::{set_file_mtime, FileTime};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// Helper function to set up a test environment
fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create some test files
    fs::write(temp_path.join("file1.txt"), "File 1 content").unwrap();
    fs::write(temp_path.join("file2.txt"), "File 2 content").unwrap();

    // Create a subdirectory with files
    fs::create_dir(temp_path.join("subdir")).unwrap();
    fs::write(temp_path.join("subdir").join("file3.txt"), "File 3 content").unwrap();

    // Create .snapsafeignore file
    fs::write(
        temp_path.join(".snapsafeignore"),
        "ignored_file.txt\nignored_dir",
    )
    .unwrap();

    // Create ignored files (should not be included in snapshots)
    fs::write(temp_path.join("ignored_file.txt"), "Should be ignored").unwrap();
    fs::create_dir(temp_path.join("ignored_dir")).unwrap();
    fs::write(
        temp_path.join("ignored_dir").join("ignored.txt"),
        "Should be ignored",
    )
    .unwrap();

    temp_dir
}

#[test]
fn test_init_command() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    let mut cmd = Command::cargo_bin("snapsafe").unwrap();
    cmd.current_dir(temp_path).arg("init").assert().success();

    assert!(temp_path.join(".snapsafe").exists());
    assert!(temp_path.join(".snapsafe").join("snapshots").exists());
    assert!(temp_path
        .join(".snapsafe")
        .join("head_manifest.json")
        .exists());
}

#[test]
fn test_snapshot_and_list() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Initialize repo
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();

    // Create snapshot
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Initial snapshot"])
        .assert()
        .success();

    // Check list output
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("v1.0.0.0"))
        .stdout(predicate::str::contains("Initial snapshot"));
}

#[test]
fn test_diff_command() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Initialize and create first snapshot
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "First snapshot"])
        .assert()
        .success();

    // Modify a file
    fs::write(temp_path.join("file1.txt"), "Modified content").unwrap();

    // Create second snapshot
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Modified file"])
        .assert()
        .success();

    // Test diff command
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["diff", "v1.0.0.0", "v1.0.0.1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"));
}

#[test]
fn test_same_size_same_timestamp_change_is_captured() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    let file_path = temp_path.join("binary.dat");
    fs::write(&file_path, b"AAAA").unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    let original_mtime = FileTime::from_last_modification_time(&fs::metadata(&file_path).unwrap());
    fs::write(&file_path, b"BBBB").unwrap();
    set_file_mtime(&file_path, original_mtime).unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    let captured = fs::read(
        temp_path
            .join(".snapsafe/snapshots/v1.0.0.1/data")
            .join("binary.dat"),
    )
    .unwrap();
    assert_eq!(captured, b"BBBB");

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["diff", "v1.0.0.0", "v1.0.0.1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("binary.dat"));
}

#[test]
fn test_verify_detects_same_size_corruption() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    let snapshot_file = temp_path
        .join(".snapsafe/snapshots/v1.0.0.0/data")
        .join("file1.txt");
    make_writable(&snapshot_file);
    fs::write(snapshot_file, "XXXXXXXXXXXXXX").unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("verify")
        .assert()
        .failure()
        .stdout(predicate::str::contains("content hash mismatch"));
}

#[test]
fn test_restore_exact_state_with_yes() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    fs::write(temp_path.join("file1.txt"), "changed content").unwrap();
    fs::write(temp_path.join("extra.txt"), "remove me").unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["--yes", "restore", "v1.0.0.0", "--no-backup"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(temp_path.join("file1.txt")).unwrap(),
        "File 1 content"
    );
    assert!(!temp_path.join("extra.txt").exists());
    assert!(temp_path.join("ignored_file.txt").exists());
    assert!(temp_path.join(".snapsafeignore").exists());
}

#[test]
fn test_restore_requires_yes_without_terminal() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();
    fs::write(temp_path.join("file1.txt"), "changed content").unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["restore", "v1.0.0.0", "--no-backup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pass --yes"));

    assert_eq!(
        fs::read_to_string(temp_path.join("file1.txt")).unwrap(),
        "changed content"
    );
}

#[test]
fn test_restore_dry_run_does_not_modify_files() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();
    fs::write(temp_path.join("file1.txt"), "changed content").unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["restore", "v1.0.0.0", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    assert_eq!(
        fs::read_to_string(temp_path.join("file1.txt")).unwrap(),
        "changed content"
    );
}

#[test]
fn test_prune_accepts_documented_duration() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["prune", "--older-than", "7d", "--dry-run"])
        .assert()
        .success();
}

#[test]
fn test_list_handles_unicode() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "--message", "1234567890123456éx"])
        .assert()
        .success();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("1234567890123456é..."));
}

#[cfg(unix)]
#[test]
fn test_snapshot_and_restore_symlink() {
    use std::os::unix::fs::symlink;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::write(temp_path.join("target-a.txt"), "a").unwrap();
    fs::write(temp_path.join("target-b.txt"), "b").unwrap();
    symlink("target-a.txt", temp_path.join("current-link")).unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .success();

    fs::remove_file(temp_path.join("current-link")).unwrap();
    symlink("target-b.txt", temp_path.join("current-link")).unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["--yes", "restore", "v1.0.0.0", "--no-backup"])
        .assert()
        .success();

    assert_eq!(
        fs::read_link(temp_path.join("current-link")).unwrap(),
        std::path::Path::new("target-a.txt")
    );
}

#[test]
fn test_corrupt_predecessor_is_not_reused() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    let first = temp_path.join(".snapsafe/snapshots/v1.0.0.0/data/file1.txt");
    make_writable(&first);
    fs::write(&first, "XXXXXXXXXXXXXX").unwrap();

    run_ok(temp_path, &["snapshot"]);
    assert_eq!(
        fs::read_to_string(temp_path.join(".snapsafe/snapshots/v1.0.0.1/data/file1.txt")).unwrap(),
        "File 1 content"
    );
    run_ok(temp_path, &["verify", "v1.0.0.1"]);
}

#[test]
fn test_restore_backup_uses_target_ignore_scope() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::write(temp_path.join("precious.bin"), "original").unwrap();

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    fs::write(temp_path.join("precious.bin"), "current!").unwrap();
    fs::write(
        temp_path.join(".snapsafeignore"),
        "ignored_file.txt\nignored_dir\nprecious.bin\n",
    )
    .unwrap();

    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("precious.bin")).unwrap(),
        "original"
    );
    assert!(fs::read_to_string(temp_path.join(".snapsafeignore"))
        .unwrap()
        .contains("precious.bin"));

    run_ok(temp_path, &["--yes", "restore", "v1.0.0.1", "--no-backup"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("precious.bin")).unwrap(),
        "current!"
    );
}

#[test]
fn test_restore_directories_types_and_mtime() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    let file_path = temp_path.join("file1.txt");
    let saved_time = FileTime::from_unix_time(946_684_800, 123_000_000);
    set_file_mtime(&file_path, saved_time).unwrap();
    fs::create_dir(temp_path.join("empty-dir")).unwrap();

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    fs::remove_file(&file_path).unwrap();
    fs::create_dir(&file_path).unwrap();
    fs::write(file_path.join("nested.txt"), "remove").unwrap();
    fs::remove_dir(temp_path.join("empty-dir")).unwrap();
    fs::remove_dir_all(temp_path.join("subdir")).unwrap();
    fs::write(temp_path.join("subdir"), "wrong type").unwrap();

    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0", "--no-backup"]);

    assert!(file_path.is_file());
    assert!(temp_path.join("subdir").is_dir());
    assert!(temp_path.join("empty-dir").is_dir());
    assert_eq!(
        FileTime::from_last_modification_time(&fs::metadata(&file_path).unwrap()),
        saved_time
    );
}

#[cfg(unix)]
#[test]
fn test_snapshot_preserves_dangling_and_cyclic_symlinks() {
    use std::os::unix::fs::symlink;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    symlink("missing-target", temp_path.join("dangling-link")).unwrap();
    symlink("cycle-link", temp_path.join("cycle-link")).unwrap();

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);
    run_ok(temp_path, &["verify"]);
    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0", "--no-backup"]);

    assert_eq!(
        fs::read_link(temp_path.join("dangling-link")).unwrap(),
        std::path::Path::new("missing-target")
    );
    assert_eq!(
        fs::read_link(temp_path.join("cycle-link")).unwrap(),
        std::path::Path::new("cycle-link")
    );
}

#[cfg(unix)]
#[test]
fn test_diff_detects_file_to_symlink_change() {
    use std::os::unix::fs::symlink;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::write(temp_path.join("shape"), "target-a.txt").unwrap();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    fs::remove_file(temp_path.join("shape")).unwrap();
    symlink("target-a.txt", temp_path.join("shape")).unwrap();
    run_ok(temp_path, &["snapshot"]);

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["diff", "v1.0.0.0", "v1.0.0.1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shape"));
}

#[cfg(unix)]
#[test]
fn test_rejects_unsupported_cross_platform_filename() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::write(temp_path.join("bad\\name"), "data").unwrap();
    run_ok(temp_path, &["init"]);

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("snapshot")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid relative path"));
}

#[cfg(unix)]
#[test]
fn test_prune_rejects_symlinked_snapshot_root() {
    use std::os::unix::fs::symlink;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    let snapshots = temp_path.join(".snapsafe/snapshots");
    let saved = temp_path.join(".snapsafe/snapshots-saved");
    fs::rename(&snapshots, &saved).unwrap();
    let external = TempDir::new().unwrap();
    fs::create_dir(external.path().join("v1.0.0.0")).unwrap();
    symlink(external.path(), &snapshots).unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["--yes", "prune", "--keep-last", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a real directory"));
    assert!(external.path().join("v1.0.0.0").exists());
}

#[test]
fn test_automatic_v1_migration() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    create_v1_repository(temp_path);

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Repository migration complete"));

    let head: serde_json::Value =
        serde_json::from_slice(&fs::read(temp_path.join(".snapsafe/head_manifest.json")).unwrap())
            .unwrap();
    assert_eq!(head["format_version"], 2);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(temp_path.join(".snapsafe/snapshots/v1.0.0.0/manifest.json")).unwrap()
        )
        .unwrap()["format_version"],
        2
    );
    run_ok(temp_path, &["verify"]);
}

#[test]
fn test_root_manifest_names_are_regular_payloads() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    for (name, content) in [
        ("manifest.json", "payload-manifest"),
        ("manifest.v1.backup", "payload-backup"),
        ("manifest.v2.tmp", "payload-sidecar"),
    ] {
        fs::write(temp_path.join(name), content).unwrap();
    }

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);
    run_ok(temp_path, &["list"]);
    run_ok(temp_path, &["verify"]);

    for name in ["manifest.json", "manifest.v1.backup", "manifest.v2.tmp"] {
        fs::remove_file(temp_path.join(name)).unwrap();
    }
    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0", "--no-backup"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("manifest.json")).unwrap(),
        "payload-manifest"
    );
    assert_eq!(
        fs::read_to_string(temp_path.join("manifest.v1.backup")).unwrap(),
        "payload-backup"
    );
    assert_eq!(
        fs::read_to_string(temp_path.join("manifest.v2.tmp")).unwrap(),
        "payload-sidecar"
    );
}

#[test]
fn test_migration_removes_ignore_rules_for_represented_files() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    create_v1_repository(temp_path);

    for version in ["v1.0.0.0", "v1.0.0.1"] {
        let folder = temp_path.join(".snapsafe/snapshots").join(version);
        fs::write(folder.join("precious.bin"), "old-snapshot").unwrap();
        let manifest_path = folder.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.as_array_mut().unwrap().push(serde_json::json!({
            "relative_path": "precious.bin",
            "file_size": 12,
            "modified": "2025-01-01 00:00:00"
        }));
        fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    }
    fs::write(temp_path.join("precious.bin"), "current-value").unwrap();
    fs::write(temp_path.join(".snapsafeignore"), "precious.bin\n").unwrap();

    run_ok(temp_path, &["list"]);
    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("precious.bin")).unwrap(),
        "old-snapshot"
    );
    run_ok(temp_path, &["--yes", "restore", "v1.0.0.2", "--no-backup"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("precious.bin")).unwrap(),
        "current-value"
    );
}

#[cfg(unix)]
#[test]
fn test_current_repository_finishes_migration_protection() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    create_v1_repository(temp_path);
    run_ok(temp_path, &["list"]);

    let file = temp_path.join(".snapsafe/snapshots/v1.0.0.0/data/file.bin");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(&file, permissions).unwrap();
    fs::create_dir_all(temp_path.join(".snapsafe/tmp/migration-v1-v2")).unwrap();

    run_ok(temp_path, &["list"]);
    assert_eq!(fs::metadata(file).unwrap().permissions().mode() & 0o200, 0);
    assert!(!temp_path.join(".snapsafe/tmp/migration-v1-v2").exists());
}

#[test]
fn test_migrates_windows_style_legacy_paths() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    let repository = temp_path.join(".snapsafe");
    let snapshot = repository.join("snapshots/v1.0.0.0");
    fs::create_dir_all(snapshot.join("dir")).unwrap();
    fs::write(snapshot.join("dir/file.bin"), "legacy-data").unwrap();
    fs::write(
        snapshot.join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!([{
            "relative_path": "dir\\file.bin",
            "file_size": 11,
            "modified": "2025-01-01 00:00:00"
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        repository.join("head_manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!([{
            "version": "v1.0.0.0",
            "timestamp": "2025-01-01 00:00:00",
            "message": null,
            "metadata": null
        }]))
        .unwrap(),
    )
    .unwrap();

    run_ok(temp_path, &["list"]);
    run_ok(temp_path, &["verify"]);
    assert_eq!(
        fs::read_to_string(snapshot.join("data/dir/file.bin")).unwrap(),
        "legacy-data"
    );
}

#[test]
fn test_migrates_legacy_top_level_data_directory() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    let repository = temp_path.join(".snapsafe");
    let snapshot = repository.join("snapshots/v1.0.0.0");
    fs::create_dir_all(snapshot.join("data")).unwrap();
    fs::write(snapshot.join("data/file.bin"), "legacy-data").unwrap();
    fs::write(
        snapshot.join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!([{
            "relative_path": "data/file.bin",
            "file_size": 11,
            "modified": "2025-01-01 00:00:00"
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        repository.join("head_manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!([{
            "version": "v1.0.0.0",
            "timestamp": "2025-01-01 00:00:00",
            "message": null,
            "metadata": null
        }]))
        .unwrap(),
    )
    .unwrap();

    run_ok(temp_path, &["list"]);
    run_ok(temp_path, &["verify"]);
    assert_eq!(
        fs::read_to_string(snapshot.join("data/data/file.bin")).unwrap(),
        "legacy-data"
    );
}

#[test]
fn test_large_file_snapshot_and_verify() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::write(temp_path.join("large.bin"), vec![0x5a; 16 * 1024 * 1024]).unwrap();

    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);
    run_ok(temp_path, &["verify"]);
}

#[test]
fn test_concurrent_snapshots_are_serialized() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);

    let binary = env!("CARGO_BIN_EXE_snapsafe");
    let mut first = std::process::Command::new(binary)
        .current_dir(temp_path)
        .arg("snapshot")
        .spawn()
        .unwrap();
    let mut second = std::process::Command::new(binary)
        .current_dir(temp_path)
        .arg("snapshot")
        .spawn()
        .unwrap();

    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    run_ok(temp_path, &["verify"]);

    let head: serde_json::Value =
        serde_json::from_slice(&fs::read(temp_path.join(".snapsafe/head_manifest.json")).unwrap())
            .unwrap();
    assert_eq!(head["snapshots"].as_array().unwrap().len(), 2);
}

#[test]
fn test_recovers_interrupted_snapshot_transaction() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);

    let temporary = temp_path.join(".snapsafe/tmp");
    fs::create_dir(temporary.join("snapshot-stale")).unwrap();
    fs::create_dir(temp_path.join(".snapsafe/snapshots/v1.0.0.0")).unwrap();
    fs::write(
        temporary.join("snapshot-v1.0.0.0.pending"),
        "v1.0.0.0\nsnapshot-stale\n",
    )
    .unwrap();

    run_ok(temp_path, &["list"]);
    assert!(!temporary.join("snapshot-stale").exists());
    assert!(!temp_path.join(".snapsafe/snapshots/v1.0.0.0").exists());
}

#[test]
fn test_recovers_interrupted_prune_transaction() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    let trash = temp_path.join(".snapsafe/tmp/prune-stale");
    fs::create_dir(&trash).unwrap();
    fs::rename(
        temp_path.join(".snapsafe/snapshots/v1.0.0.0"),
        trash.join("v1.0.0.0"),
    )
    .unwrap();

    run_ok(temp_path, &["list"]);
    assert!(temp_path.join(".snapsafe/snapshots/v1.0.0.0").is_dir());
    assert!(!trash.exists());
    run_ok(temp_path, &["verify"]);
}

#[test]
fn test_prune_deletes_only_selected_snapshots() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);
    fs::write(temp_path.join("file1.txt"), "next version").unwrap();
    run_ok(temp_path, &["snapshot"]);

    run_ok(temp_path, &["--yes", "prune", "--keep-last", "1"]);
    assert!(!temp_path.join(".snapsafe/snapshots/v1.0.0.0").exists());
    assert!(temp_path.join(".snapsafe/snapshots/v1.0.0.1").exists());
    run_ok(temp_path, &["verify"]);
}

#[test]
fn test_rejects_unsafe_manifest_path() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    let manifest_path = temp_path.join(".snapsafe/snapshots/v1.0.0.0/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"][0]["relative_path"] = serde_json::json!("../outside");
    fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("verify")
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid relative path"));
}

#[test]
fn test_rejects_unknown_repository_format() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    run_ok(temp_path, &["init"]);
    fs::write(
        temp_path.join(".snapsafe/head_manifest.json"),
        r#"{"format_version":99,"snapshots":[]}"#,
    )
    .unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported head manifest format"));
}

#[cfg(unix)]
#[test]
fn test_restore_replaces_parent_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    fs::create_dir(temp_path.join("safe")).unwrap();
    fs::write(temp_path.join("safe/file.txt"), "snapshot").unwrap();
    run_ok(temp_path, &["init"]);
    run_ok(temp_path, &["snapshot"]);

    fs::remove_dir_all(temp_path.join("safe")).unwrap();
    let external = TempDir::new().unwrap();
    fs::write(external.path().join("file.txt"), "external").unwrap();
    symlink(external.path(), temp_path.join("safe")).unwrap();

    run_ok(temp_path, &["--yes", "restore", "v1.0.0.0", "--no-backup"]);
    assert_eq!(
        fs::read_to_string(temp_path.join("safe/file.txt")).unwrap(),
        "snapshot"
    );
    assert_eq!(
        fs::read_to_string(external.path().join("file.txt")).unwrap(),
        "external"
    );
}

#[test]
fn test_resumed_migration_validates_prepared_payload() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    create_v1_repository(temp_path);

    let migration = temp_path.join(".snapsafe/tmp/migration-v1-v2/v1.0.0.0");
    fs::create_dir_all(migration.join("data")).unwrap();
    fs::copy(
        temp_path.join(".snapsafe/snapshots/v1.0.0.0/manifest.json"),
        migration.join("manifest.v1"),
    )
    .unwrap();
    fs::write(migration.join("data/file.bin"), "badbadbad!!").unwrap();
    let digest = blake3::hash(b"legacy-data").to_hex().to_string();
    fs::write(
        migration.join("manifest.v2"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "format_version": 2,
            "ignore": [],
            "files": [{
                "relative_path": "file.bin",
                "file_size": 11,
                "modified": "2025-01-01T00:00:00.000000000Z",
                "kind": "file",
                "content_hash": {"algorithm": "blake3", "digest": digest},
                "link_target": null,
                "unix_mode": null
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("migrated snapshot file mismatch"));
    assert_eq!(
        fs::read_to_string(temp_path.join(".snapsafe/snapshots/v1.0.0.0/file.bin")).unwrap(),
        "legacy-data"
    );
    let head: serde_json::Value =
        serde_json::from_slice(&fs::read(temp_path.join(".snapsafe/head_manifest.json")).unwrap())
            .unwrap();
    assert!(head.is_array());
}

fn run_ok(path: &std::path::Path, arguments: &[&str]) {
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(path)
        .args(arguments)
        .assert()
        .success();
}

fn create_v1_repository(base: &std::path::Path) {
    let repository = base.join(".snapsafe");
    let snapshots = repository.join("snapshots");
    fs::create_dir_all(&snapshots).unwrap();

    let versions = ["v1.0.0.0", "v1.0.0.1"];
    for version in versions {
        let folder = snapshots.join(version);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("file.bin"), "legacy-data").unwrap();
        fs::write(
            folder.join("manifest.json"),
            serde_json::to_vec_pretty(&serde_json::json!([{
                "relative_path": "file.bin",
                "file_size": 11,
                "modified": "2025-01-01 00:00:00"
            }]))
            .unwrap(),
        )
        .unwrap();
    }

    fs::remove_file(snapshots.join("v1.0.0.1/file.bin")).unwrap();
    fs::hard_link(
        snapshots.join("v1.0.0.0/file.bin"),
        snapshots.join("v1.0.0.1/file.bin"),
    )
    .unwrap();

    fs::write(
        repository.join("head_manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!([
            {
                "version": "v1.0.0.0",
                "timestamp": "2025-01-01 00:00:00",
                "message": "first",
                "metadata": null
            },
            {
                "version": "v1.0.0.1",
                "timestamp": "2025-01-02 00:00:00",
                "message": "second",
                "metadata": null
            }
        ]))
        .unwrap(),
    )
    .unwrap();
}

#[cfg(unix)]
fn make_writable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(windows)]
fn make_writable(path: &std::path::Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn test_tag_and_metadata() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();

    // Initialize and create snapshot
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Tagged snapshot"])
        .assert()
        .success();

    // Add tags
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["tag", "v1.0.0.0", "--add", "test-tag", "another-tag"])
        .assert()
        .success();

    // Add metadata
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .args(["meta", "v1.0.0.0", "--set", "test-key", "test-value"])
        .assert()
        .success();

    // Verify tags and metadata appear in list
    Command::cargo_bin("snapsafe")
        .unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-tag"))
        .stdout(predicate::str::contains("test-key=test-value"));
}

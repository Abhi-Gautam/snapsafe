use assert_cmd::Command;
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
    fs::write(temp_path.join(".snapsafeignore"), "ignored_file.txt\nignored_dir").unwrap();
    
    // Create ignored files (should not be included in snapshots)
    fs::write(temp_path.join("ignored_file.txt"), "Should be ignored").unwrap();
    fs::create_dir(temp_path.join("ignored_dir")).unwrap();
    fs::write(temp_path.join("ignored_dir").join("ignored.txt"), "Should be ignored").unwrap();
    
    temp_dir
}

#[test]
fn test_init_command() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    let mut cmd = Command::cargo_bin("snapsafe").unwrap();
    cmd.current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    assert!(temp_path.join(".snapsafe").exists());
    assert!(temp_path.join(".snapsafe").join("snapshots").exists());
    assert!(temp_path.join(".snapsafe").join("head_manifest.json").exists());
}

#[test]
fn test_snapshot_and_list() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Create snapshot
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Initial snapshot"])
        .assert()
        .success();
    
    // Check list output
    Command::cargo_bin("snapsafe").unwrap()
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
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "First snapshot"])
        .assert()
        .success();
    
    // Modify a file
    fs::write(temp_path.join("file1.txt"), "Modified content").unwrap();
    
    // Create second snapshot
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Modified file"])
        .assert()
        .success();
    
    // Test diff command
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["diff", "v1.0.0.0", "v1.0.0.1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"));
}

#[test]
fn test_tag_and_metadata() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Initialize and create snapshot
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Tagged snapshot"])
        .assert()
        .success();
    
    // Add tags
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["tag", "v1.0.0.0", "--add", "test-tag", "another-tag"])
        .assert()
        .success();
    
    // Add metadata
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["meta", "v1.0.0.0", "--set", "test-key", "test-value"])
        .assert()
        .success();
    
    // Verify tags and metadata appear in list
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-tag"))
        .stdout(predicate::str::contains("test-key=test-value"));
}
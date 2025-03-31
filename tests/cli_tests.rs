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

// Helper function to clean up global config for tests
fn clean_global_config() {
    if let Some(config_dir_base) = dirs::config_dir() {
        let config_dir = config_dir_base.join("snapsafe");
        let config_file = config_dir.join("config.json");
        
        if config_file.exists() {
            let _ = fs::remove_file(config_file);
        }
        
        if config_dir.exists() {
            let _ = fs::remove_dir_all(config_dir);
        }
    }
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

#[test]
fn test_global_config_creation() {
    clean_global_config();
    
    // Directly check what's there before our test
    let before = Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--list"])
        .output()
        .unwrap();
    println!("BEFORE: {}", String::from_utf8(before.stdout).unwrap());
    
    // Set a global config value with unique test value
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "text_diff_extensions", "rs,toml,md,TEST_MARKER"])
        .assert()
        .success();
    
    // Immediately get the value back to verify
    let output = Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--get", "text_diff_extensions"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    println!("AFTER: {}", output_str);
    
    // Check for our unique test marker
    assert!(output_str.contains("TEST_MARKER"), 
           "Expected to find TEST_MARKER in output: {}", output_str);
    
    clean_global_config();
}

#[test]
fn test_repo_config_creation() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Set a repo config value
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--set", "text_diff_extensions", "js,html,css"])
        .assert()
        .success();
    
    // Check that the repo config file was created
    let config_file = temp_path.join(".snapsafe").join("config.json");
    assert!(config_file.exists());
    
    // Check the content
    let content = fs::read_to_string(config_file).unwrap();
    assert!(content.contains("text_diff_extensions"));
    assert!(content.contains("js,html,css"));
}

#[test]
fn test_config_precedence() {
    clean_global_config();
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Set global config
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "text_diff_extensions", "global,values"])
        .assert()
        .success();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Set repo config
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--set", "text_diff_extensions", "repo,values"])
        .assert()
        .success();
    
    // Get the value - should show repo value
    let output = Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--get", "text_diff_extensions"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("repo,values"));
    assert!(output_str.contains("(repository)"));
    assert!(!output_str.contains("global,values"));
    
    clean_global_config();
}

#[test]
fn test_config_fallback() {
    clean_global_config();
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Set global config with a distinctive value
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "text_diff_extensions", "global,values,test"])
        .assert()
        .success();
    
    // Verify global setting was set
    let output = Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--get", "text_diff_extensions"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    println!("Output: {}", output_str); // Debug output
    assert!(output_str.contains("global,values,test"));
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Get the value from repo context - should fall back to global
    let output = Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--get", "text_diff_extensions"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    println!("Output: {}", output_str); // Debug output
    assert!(output_str.contains("global,values,test"));
    
    clean_global_config();
}

#[test]
fn test_config_list_command() {
    clean_global_config();
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Set global config
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "text_diff_extensions", "global,ext"])
        .assert()
        .success();
    
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "ignore_list", "global,ignore"])
        .assert()
        .success();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Set repo config for just one key
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--set", "text_diff_extensions", "repo,ext"])
        .assert()
        .success();
    
    // List configs - should show both repo and global
    let output = Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--list"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    println!("Output: {}", output_str); // Debug output
    assert!(output_str.contains("Repository configuration settings"));
    assert!(output_str.contains("Global default settings"));
    assert!(output_str.contains("text_diff_extensions"));
    assert!(output_str.contains("repo,ext"));
    assert!(output_str.contains("ignore_list"));
    assert!(output_str.contains("global,ignore"));
    
    clean_global_config();
}

#[test]
fn test_custom_ignore_list() {
    clean_global_config();
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Set global config with custom ignore list
    Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--set", "ignore_list", "custom_dir,*.log,*.tmp"])
        .assert()
        .success();
    
    // Verify global setting was applied
    let output = Command::cargo_bin("snapsafe").unwrap()
        .args(["config", "--global", "--get", "ignore_list"])
        .output()
        .unwrap();
    
    let output_str = String::from_utf8(output.stdout).unwrap();
    assert!(output_str.contains("custom_dir"));
    
    // Now initialize the repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Check the ignore file
    let ignore_file = temp_path.join(".snapsafeignore");
    assert!(ignore_file.exists());
    
    let ignore_content = fs::read_to_string(ignore_file).unwrap();
    println!("{}", ignore_content);
    assert!(ignore_content.contains("custom_dir"));
    
    clean_global_config();
}

#[test]
fn test_snapshot_with_custom_ignore() {
    clean_global_config();
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Create test directories and files
    fs::create_dir(temp_path.join("custom_ignore_dir")).unwrap();
    fs::write(temp_path.join("custom_ignore_dir").join("ignored.txt"), "This should be ignored").unwrap();
    fs::write(temp_path.join("normal_file.txt"), "This should be included").unwrap();
    
    // Set custom ignore list in repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Manually create custom .snapsafeignore
    fs::write(temp_path.join(".snapsafeignore"), "custom_ignore_dir").unwrap();
    
    // Create snapshot
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["snapshot", "-m", "Test ignore"])
        .assert()
        .success();
    
    // Verify the snapshot doesn't include ignored files
    let snapshot_dir = temp_path
        .join(".snapsafe")
        .join("snapshots")
        .join("v1.0.0.0");
    
    assert!(snapshot_dir.join("normal_file.txt").exists());
    assert!(!snapshot_dir.join("custom_ignore_dir").exists());
    
    clean_global_config();
}

#[test]
fn test_invalid_config_key() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Try to set an invalid config key
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--set", "invalid_key", "some_value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown configuration key"));
}

#[test]
fn test_invalid_config_value() {
    let temp_dir = setup_test_env();
    let temp_path = temp_dir.path();
    
    // Initialize repo
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .arg("init")
        .assert()
        .success();
    
    // Try to set an empty value (which is invalid)
    Command::cargo_bin("snapsafe").unwrap()
        .current_dir(temp_path)
        .args(["config", "--set", "text_diff_extensions", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}
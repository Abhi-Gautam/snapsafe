use std::fs;
use std::io;
use std::path::Path;

use crate::constants::{CONFIG_FILE, REPO_FOLDER};
use crate::info;
use crate::models::SnapsafeConfig;

/// Configure Snap Safe settings
pub fn configure(set: Option<Vec<String>>, get: Option<String>, list: bool) -> io::Result<()> {
    let base_path = info::get_base_dir()?;
    let repo_path = base_path.join(REPO_FOLDER);
    
    if !repo_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Repository not initialized. Please run the init command first."
        ));
    }
    
    let config_path = repo_path.join(CONFIG_FILE);
    let mut config = load_config(&config_path)?;
    
    if let Some(values) = set {
        if values.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Please provide exactly two values for --set: a key and a value."
            ));
        }
        
        let key = &values[0];
        let value = &values[1];
        
        // Validate the key and value
        if !is_valid_config_key(key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unknown configuration key: {}. Run with --list to see available keys.", key)
            ));
        }
        
        if !is_valid_config_value(key, value) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid value for {}: {}. See documentation for valid values.", key, value)
            ));
        }
        
        config.settings.insert(key.clone(), value.clone());
        save_config(&config_path, &config)?;
        println!("Configuration updated: {} = {}", key, value);
    } else if let Some(key) = get {
        if !is_valid_config_key(&key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unknown configuration key: {}. Run with --list to see available keys.", key)
            ));
        }
        
        match config.settings.get(&key) {
            Some(value) => println!("{} = {}", key, value),
            None => println!("{} is not set", key),
        }
    } else if list {
        println!("Available configuration settings:");
        println!("{:-<50}", "");
        
        let mut settings: Vec<(String, String)> = config.settings.into_iter().collect();
        settings.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (key, value) in settings {
            println!("{:<20} = {}", key, value);
        }
    } else {
        println!("No configuration option provided. Use --set, --get, or --list.");
    }
    
    Ok(())
}

/// Load the configuration from the config file
fn load_config(config_path: &Path) -> io::Result<SnapsafeConfig> {
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        match serde_json::from_str::<SnapsafeConfig>(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("Warning: Could not parse config file: {}", e);
                Ok(SnapsafeConfig::default())
            }
        }
    } else {
        let config = SnapsafeConfig::default();
        save_config(config_path, &config)?;
        Ok(config)
    }
}

/// Save the configuration to the config file
fn save_config(config_path: &Path, config: &SnapsafeConfig) -> io::Result<()> {
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    fs::write(config_path, content)
}

/// Check if a configuration key is valid
fn is_valid_config_key(key: &str) -> bool {
    let valid_keys = [
        "autobackup",
        "compression",
        "default_snapshot_message",
        "max_backups",
        "verify_after_snapshot",
        "text_diff_extensions",
    ];
    
    valid_keys.contains(&key)
}

/// Check if a configuration value is valid for the given key
fn is_valid_config_value(key: &str, value: &str) -> bool {
    match key {
        "autobackup" => ["true", "false"].contains(&value),
        "compression" => ["none", "fast", "best"].contains(&value),
        "max_backups" => value.parse::<usize>().is_ok(),
        "verify_after_snapshot" => ["true", "false"].contains(&value),
        // For text_diff_extensions, any comma-separated list is valid
        "text_diff_extensions" => {
            !value.is_empty() && value.split(',').all(|ext| !ext.trim().is_empty())
        },
        // For other keys, any non-empty string is valid
        _ => !value.is_empty(),
    }
}
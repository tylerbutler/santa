//! Integration tests for configuration hot-reload behavior
//!
//! These tests validate that Santa properly handles configuration changes,
//! including hot-reloading when supported and graceful fallback when not.

use santa::configuration::SantaConfig;
use santa::data::KnownSources;
use santa::traits::Configurable;
use std::fs;
use tempfile::{NamedTempFile, TempDir};

#[tokio::test]
async fn test_config_load_and_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("santa.yaml");

    let config_content = r#"
sources:
  - apt
packages:
  - curl
custom_sources:
  - name: "test-source"
    emoji: "🧪"
    shell_command: "test"
    install_command: "test install"
    check_command: "test list"
    prepend_to_package_name: null
    overrides: null
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let result = SantaConfig::load_config(&config_path);
    assert!(
        result.is_ok(),
        "Config loading should succeed: {:?}",
        result
    );

    let config = result.unwrap();
    assert!(!config.sources.is_empty());
    assert!(!config.packages.is_empty());
    assert!(config.custom_sources.is_some());

    let custom_sources = config.custom_sources.unwrap();
    assert_eq!(custom_sources.len(), 1);
    assert_eq!(custom_sources[0].name_str(), "test-source");
}

#[tokio::test]
async fn test_invalid_config_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid.yaml");

    let invalid_config = r#"
invalid_yaml: [
  unclosed_bracket
"#;

    fs::write(&config_path, invalid_config).expect("Failed to write invalid config");

    let result = SantaConfig::load_config(&config_path);
    assert!(result.is_err(), "Invalid config should fail to load");
}

#[tokio::test]
async fn test_missing_config_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nonexistent_path = temp_dir.path().join("nonexistent.yaml");

    let result = SantaConfig::load_config(&nonexistent_path);
    assert!(result.is_err(), "Missing config file should cause error");
}

#[tokio::test]
async fn test_config_validation() {
    // Valid config
    let valid_config = SantaConfig {
        sources: vec![KnownSources::Apt],
        packages: vec!["curl".to_string()],
        custom_sources: None,
        _groups: None,
        log_level: 0,
    };

    let result = SantaConfig::validate_config(&valid_config);
    assert!(result.is_ok(), "Valid config should pass validation");
}

#[tokio::test]
async fn test_hot_reload_capability() {
    let config = SantaConfig {
        sources: vec![KnownSources::Apt],
        packages: vec!["curl".to_string()],
        custom_sources: None,
        _groups: None,
        log_level: 0,
    };

    // Santa config should support hot reload
    assert!(config.hot_reload_supported());
}

#[tokio::test]
async fn test_config_with_custom_sources() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("custom_sources.yaml");

    let config_content = r#"
sources:
  - apt
packages:
  - curl
custom_sources:
  - name: "custom-apt"
    emoji: "📦"
    shell_command: "apt"
    install_command: "apt install"
    check_command: "apt list --installed"
    prepend_to_package_name: null
    overrides: null
  - name: "custom-snap"
    emoji: "🚀"
    shell_command: "snap"
    install_command: "snap install"
    check_command: "snap list"
    prepend_to_package_name: null
    overrides: null
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    let result = SantaConfig::load_config(&config_path);
    assert!(result.is_ok(), "Config with custom sources should load");

    let config = result.unwrap();
    let custom_sources = config.custom_sources.expect("Should have custom sources");

    assert_eq!(custom_sources.len(), 2);
    assert_eq!(custom_sources[0].name_str(), "custom-apt");
    assert_eq!(custom_sources[1].name_str(), "custom-snap");
    assert_eq!(custom_sources[0].install_command(), "apt install");
    assert_eq!(custom_sources[1].install_command(), "snap install");
}

#[tokio::test]
async fn test_config_update_detection() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let config_path = temp_file.path();

    // Write initial config
    let initial_config = r#"
sources:
  - apt
packages:
  - curl
custom_sources: []
"#;
    fs::write(config_path, initial_config).expect("Failed to write initial config");

    let initial_result = SantaConfig::load_config(config_path);
    assert!(initial_result.is_ok());

    let initial_config = initial_result.unwrap();
    assert_eq!(initial_config.custom_sources.unwrap().len(), 0);

    // Update config
    let updated_config = r#"
sources:
  - apt
packages:
  - curl
custom_sources:
  - name: "new-source"
    emoji: "🆕"
    shell_command: "new"
    install_command: "new install"
    check_command: "new list"
    prepend_to_package_name: null
    overrides: null
"#;
    fs::write(config_path, updated_config).expect("Failed to write updated config");

    let updated_result = SantaConfig::load_config(config_path);
    assert!(updated_result.is_ok());

    let updated_config = updated_result.unwrap();
    assert_eq!(updated_config.custom_sources.unwrap().len(), 1);
}

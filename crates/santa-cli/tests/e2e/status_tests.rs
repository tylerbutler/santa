//! End-to-end tests for the `status` command
//!
//! Tests verify CLI behavior for package status checking with and without flags.

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn status_command_with_builtin_config() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);

    // Should succeed and show status information
    cmd.assert().success();
}

#[test]
fn status_command_with_all_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--all", "--builtin-only"]);

    // Should succeed and show all packages
    cmd.assert().success();
}

#[test]
fn status_command_default_behavior() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);

    // Default behavior (without --all) should show missing packages
    cmd.assert().success();
}

#[test]
fn status_command_with_verbose() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only", "-v"]);

    // Should succeed with verbose output
    cmd.assert().success();
}

#[test]
fn status_command_with_custom_config() {
    use std::io::Write;

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r#"sources =
  = brew

packages =
  = git
  = curl
"#
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.arg("status");

    // Should process custom config successfully
    cmd.assert().success();
}

#[test]
fn status_command_output_format() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);

    // Output should be well-formatted (not checking specific content)
    let output = cmd.output().unwrap();
    assert!(output.status.success());

    // Should produce some output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Status should produce output");
}

#[test]
fn status_command_with_multiple_verbosity_levels() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only", "-vv"]);

    // Should handle multiple verbosity flags
    cmd.assert().success();
}

#[test]
fn status_command_exit_codes() {
    // Test that status command exits with success code
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["status", "--builtin-only"]);

    cmd.assert().success().code(0);
}

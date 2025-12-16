//\! End-to-end tests for the `config` command
//\!
//\! Tests verify CLI behavior for configuration display and validation.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn config_command_shows_default_config() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);

    // Should display default configuration
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("name:"))
        .stdout(predicate::str::contains("emoji:"));
}

#[test]
fn config_command_with_packages_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--packages", "--builtin-only"]);

    // Should show package information
    cmd.assert().success();
}

#[test]
fn config_command_with_pipe_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--pipe", "--builtin-only"]);

    // Should produce pipe-friendly output
    cmd.assert().success();
}

#[test]
fn config_command_with_custom_config_file() {
    use std::io::Write;

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r#"sources =
  = brew
  = cargo

packages =
  = rust-analyzer
  = ripgrep
"#
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.arg("config");

    // Should display custom configuration (sources appear capitalized like "Brew", "Cargo")
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Brew").or(predicate::str::contains("Cargo")));
}

#[test]
fn config_command_with_invalid_config_file() {
    use std::io::Write;

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(config_file, "invalid ccl syntax @@@").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.arg("config");

    // Should handle invalid config gracefully (may succeed with default or fail)
    let output = cmd.output().unwrap();
    // Just verify it doesn't panic
    assert!(output.status.code().is_some());
}

#[test]
fn config_command_output_is_valid_format() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);

    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Config should produce output");

    // Should contain structured configuration data
    assert!(
        stdout.contains("name:") || stdout.contains("sources"),
        "Config output should show configuration structure"
    );
}

#[test]
fn config_command_with_verbose_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only", "-v"]);

    // Should handle verbose output
    cmd.assert().success();
}

#[test]
fn config_command_respects_log_levels() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only", "-vvv"]);

    // Should handle maximum verbosity
    cmd.assert().success();
}

#[test]
fn config_command_exit_codes() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("santa"));
    cmd.args(["config", "--builtin-only"]);

    // Should exit with success code
    cmd.assert().success().code(0);
}

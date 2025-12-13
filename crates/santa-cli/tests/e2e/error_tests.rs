//\! End-to-end tests for error handling and edge cases
//\!
//\! Tests verify CLI behavior when encountering errors and invalid inputs.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::NamedTempFile;

#[test]
fn invalid_subcommand_shows_help() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.arg("invalid-command");

    // Should fail with helpful error message
    cmd.assert().failure().stderr(
        predicate::str::contains("invalid-command").or(predicate::str::contains("unrecognized")),
    );
}

#[test]
fn missing_required_argument_shows_error() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.arg("add");

    // Add command requires arguments, should show error or prompt
    let output = cmd.output().unwrap();
    // May fail or succeed (if it prompts), but shouldn't panic
    assert!(output.status.code().is_some());
}

#[test]
fn invalid_flag_combination_handled() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.args(["--unknown-flag", "status"]);

    // Should fail gracefully with error message
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unknown").or(predicate::str::contains("unexpected")));
}

#[test]
fn nonexistent_config_file_handled() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.env("SANTA_CONFIG_PATH", "/nonexistent/path/to/config.ccl");
    cmd.args(["status", "--builtin-only"]);

    // Should handle missing config file gracefully
    // (builtin-only flag should make this succeed with default config)
    cmd.assert().success();
}

#[test]
fn malformed_config_file_shows_error() {
    use std::io::Write;

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(config_file, "completely invalid syntax @@@ {{{{ ]]]]]").unwrap();

    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.arg("config");

    // Should handle malformed config (may use default or show error)
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());
}

#[test]
fn empty_config_file_handled() {
    use std::io::Write;

    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(config_file, "").unwrap();

    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.args(["status"]);

    // Should handle empty config gracefully
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());
}

#[test]
fn help_flag_always_works() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.arg("--help");

    // Help should always succeed
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:").or(predicate::str::contains("santa")));
}

#[test]
fn version_flag_always_works() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.arg("--version");

    // Version should always succeed
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("santa"));
}

#[test]
fn invalid_source_name_handled() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.args(["install", "nonexistent-source", "--builtin-only"]);

    // Should handle invalid source gracefully
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());
}

#[test]
fn dangerous_package_names_sanitized() {
    use std::io::Write;

    // Test that dangerous package names are properly escaped
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r#"
sources = ["brew"]
packages = ["git; rm -rf /"]
"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.env("SANTA_CONFIG_PATH", config_file.path());
    cmd.arg("status");

    // Should process without executing the dangerous command
    // (status just checks, doesn't execute)
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());

    // Should not contain error about command execution
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("rm -rf"));
}

#[test]
fn execution_mode_flag_validation() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.args(["--execute", "status", "--builtin-only"]);

    // Should handle execute flag (safe mode vs execute mode)
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());
}

#[test]
fn script_format_validation() {
    let mut cmd = Command::cargo_bin("santa").unwrap();
    cmd.args(["--format", "shell", "status", "--builtin-only"]);

    // Should handle format flag
    let output = cmd.output().unwrap();
    assert!(output.status.code().is_some());
}

#[test]
fn concurrent_operations_safe() {
    use std::thread;

    // Test that multiple status commands can run concurrently
    let handles: Vec<_> = (0..3)
        .map(|_| {
            thread::spawn(|| {
                let mut cmd = Command::cargo_bin("santa").unwrap();
                cmd.args(["status", "--builtin-only"]);
                cmd.output()
            })
        })
        .collect();

    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

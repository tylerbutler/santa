//! Snapshot tests for error messages with hints
//!
//! These tests verify that error messages include actionable context
//! to help users resolve common issues.

use assert_cmd::Command;
use predicates::prelude::*;

fn santa_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

#[test]
fn error_missing_config_shows_error_with_context() {
    santa_cmd()
        .args(["status"])
        .env("SANTA_CONFIG_PATH", "/nonexistent/path/config.ccl")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Config")));
}

#[test]
fn error_add_unknown_package_shows_hint() {
    santa_cmd()
        .args(["add", "this-package-definitely-does-not-exist-xyz"])
        .env("SANTA_CONFIG_PATH", "/nonexistent/path/config.ccl")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn error_add_no_packages_shows_usage() {
    santa_cmd()
        .args(["add"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No packages specified")
                .or(predicate::str::contains("Usage")),
        );
}

#[test]
fn error_remove_no_packages_shows_usage() {
    santa_cmd()
        .args(["remove"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No packages specified")
                .or(predicate::str::contains("Usage")),
        );
}

#[test]
fn error_sources_show_unknown_source() {
    santa_cmd()
        .args(["sources", "show", "nonexistent-manager"])
        .args(["--builtin-only"])
        .assert()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn error_malformed_config_shows_caused_by() {
    santa_cmd()
        .args(["status"])
        .env("SANTA_CONFIG_PATH", "/dev/null")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("caused by:"));
}

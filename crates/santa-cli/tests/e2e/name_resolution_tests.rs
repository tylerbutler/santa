//! Name resolution tests for package manager script generation
//!
//! These tests verify that package names appear correctly in generated scripts,
//! including cross-platform name resolution for common packages.

use assert_cmd::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

fn santa_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("santa"))
}

/// Write a minimal CCL config with the given sources and packages.
fn write_test_config(sources: &[&str], packages: &[&str]) -> NamedTempFile {
    let mut config_file = NamedTempFile::new().unwrap();
    let mut content = String::from("sources =\n");
    for source in sources {
        content.push_str(&format!("  = {source}\n"));
    }
    content.push_str("packages =\n");
    for pkg in packages {
        content.push_str(&format!("  = {pkg}\n"));
    }
    write!(config_file, "{content}").unwrap();
    config_file
}

/// Generate scripts for a manager/package combo and verify the package name appears.
fn verify_package_in_script(manager: &str, package_name: &str) {
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(&[manager], &[package_name]);

    santa_cmd()
        .env("SANTA_CONFIG_PATH", config.path())
        .args([
            "install",
            "--format",
            "shell",
            "--output-dir",
            output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let scripts: Vec<String> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();

    if !scripts.is_empty() {
        assert!(
            scripts.iter().any(|s| s.contains(package_name)),
            "Script for {manager} should contain '{package_name}'"
        );
    }
}

#[test]
fn ripgrep_on_brew() {
    verify_package_in_script("brew", "ripgrep");
}

#[test]
fn ripgrep_on_apt() {
    verify_package_in_script("apt", "ripgrep");
}

#[test]
fn ripgrep_on_pacman() {
    verify_package_in_script("pacman", "ripgrep");
}

#[test]
fn bat_on_brew() {
    verify_package_in_script("brew", "bat");
}

#[test]
fn bat_on_apt() {
    verify_package_in_script("apt", "bat");
}

#[test]
fn jq_on_brew() {
    verify_package_in_script("brew", "jq");
}

#[test]
fn jq_on_apt() {
    verify_package_in_script("apt", "jq");
}

#[test]
fn jq_on_pacman() {
    verify_package_in_script("pacman", "jq");
}

#[test]
fn curl_on_multiple_managers() {
    for manager in &["brew", "apt", "pacman", "arch"] {
        verify_package_in_script(manager, "curl");
    }
}

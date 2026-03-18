//! Tier 1 manager script generation tests
//!
//! Tests verify that the `install` command generates correct shell scripts
//! for each Tier 1 package manager (brew, apt, pacman, cargo, npm).

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

/// Read all .sh files from the given directory and return their contents.
fn get_generated_scripts(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect()
}

#[test]
fn brew_script_contains_brew_install() {
    let output_dir = TempDir::new().unwrap();
    // apktool and argocd are available for brew in the package database
    // (using uncommon packages to avoid filtering by already-installed check)
    let config = write_test_config(&["brew"], &["apktool", "argocd"]);

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

    let scripts = get_generated_scripts(output_dir.path());
    assert!(!scripts.is_empty(), "No .sh scripts generated for brew");
    assert!(
        scripts.iter().any(|s| s.contains("brew install")),
        "brew script should contain 'brew install'"
    );
}

#[test]
fn apt_script_contains_apt_install() {
    let output_dir = TempDir::new().unwrap();
    // ripgrep and bat are available for apt in the package database
    let config = write_test_config(&["apt"], &["ripgrep", "bat"]);

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

    let scripts = get_generated_scripts(output_dir.path());
    assert!(!scripts.is_empty(), "No .sh scripts generated for apt");
    assert!(
        scripts.iter().any(|s| s.contains("apt install")),
        "apt script should contain 'apt install'"
    );
}

#[test]
fn pacman_script_contains_pacman() {
    let output_dir = TempDir::new().unwrap();
    // ripgrep and bat are available for pacman in the package database
    let config = write_test_config(&["pacman"], &["ripgrep", "bat"]);

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

    let scripts = get_generated_scripts(output_dir.path());
    assert!(!scripts.is_empty(), "No .sh scripts generated for pacman");
    assert!(
        scripts.iter().any(|s| s.contains("pacman")),
        "pacman script should contain 'pacman'"
    );
}

#[test]
fn cargo_script_contains_cargo_install() {
    let output_dir = TempDir::new().unwrap();
    // Use multiple cargo packages — at least one should be uninstalled on any machine
    let config = write_test_config(&["cargo"], &["dotenv-linter", "hurl", "procs", "xsv"]);

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

    let scripts = get_generated_scripts(output_dir.path());
    assert!(!scripts.is_empty(), "No .sh scripts generated for cargo");
    assert!(
        scripts.iter().any(|s| s.contains("cargo install")),
        "cargo script should contain 'cargo install'"
    );
}

#[test]
fn npm_script_contains_npm_install() {
    let output_dir = TempDir::new().unwrap();
    // npkill, npm-check-updates, yarn are available for npm in the package database
    let config = write_test_config(&["npm"], &["npkill", "yarn"]);

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

    let scripts = get_generated_scripts(output_dir.path());
    assert!(!scripts.is_empty(), "No .sh scripts generated for npm");
    assert!(
        scripts.iter().any(|s| s.contains("npm install")),
        "npm script should contain 'npm install'"
    );
}

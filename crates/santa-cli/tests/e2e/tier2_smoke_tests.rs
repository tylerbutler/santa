//! Tier 2 smoke tests for package manager script generation
//!
//! These tests verify that the `install` command does not crash for Tier 2
//! package managers (dnf, nix, arch, aur, scoop, winget, flathub).
//! They do not assert script content, only that the command succeeds.

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

macro_rules! tier2_smoke_test {
    ($name:ident, $sources:expr, $packages:expr) => {
        #[test]
        fn $name() {
            let output_dir = TempDir::new().unwrap();
            let config = write_test_config($sources, $packages);
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
        }
    };
}

// dnf has no packages in the database, but the command should still succeed
tier2_smoke_test!(dnf_does_not_crash, &["dnf"], &["git", "curl"]);

// nix has packages like atuin, ripgrep in the database
tier2_smoke_test!(nix_does_not_crash, &["nix"], &["atuin", "ripgrep"]);

// arch has packages like binutils, curl in the database
tier2_smoke_test!(arch_does_not_crash, &["arch"], &["binutils", "curl"]);

// aur has packages like add-gitignore, ack in the database
tier2_smoke_test!(aur_does_not_crash, &["aur"], &["add-gitignore", "ack"]);

// scoop has packages like git, curl in the database
tier2_smoke_test!(scoop_does_not_crash, &["scoop"], &["git", "curl"]);

// winget has no packages in the database
tier2_smoke_test!(winget_does_not_crash, &["winget"], &["git"]);

// flathub has no packages in the database
tier2_smoke_test!(flathub_does_not_crash, &["flathub"], &["firefox"]);

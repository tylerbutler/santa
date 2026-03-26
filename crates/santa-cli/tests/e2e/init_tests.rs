use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn santa_init_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

#[test]
fn init_with_yes_creates_config() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config written to"));

    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("sources ="));
    assert!(content.contains("packages ="));
}

#[test]
fn init_refuses_overwrite_with_yes_flag() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");
    std::fs::write(&config_path, "existing =\n").unwrap();

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_generates_cli_loadable_config() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .success();

    santa_init_cmd()
        .args(["--config", config_path.to_str().unwrap(), "config"])
        .assert()
        .success();
}

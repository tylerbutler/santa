use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use std::path::PathBuf;

#[allow(deprecated)]
fn sickle() -> Command {
    Command::cargo_bin("sickle").unwrap()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn convert_ccl_to_json() {
    sickle()
        .args(["convert", fixture("sample.ccl").to_str().unwrap(), "--to", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"MyApp\""));
}

#[test]
fn convert_json_to_ccl() {
    sickle()
        .args(["convert", "--from", "json", "--to", "ccl"])
        .write_stdin("{\"name\": \"Alice\"}")
        .assert()
        .success()
        .stdout(predicate::str::contains("name = Alice"));
}

#[test]
fn convert_ccl_to_toml() {
    sickle()
        .args(["convert", fixture("sample.ccl").to_str().unwrap(), "--to", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name = "));
}

#[test]
fn convert_requires_to_flag() {
    sickle()
        .args(["convert", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--to"));
}

#[test]
fn convert_stdin_requires_from() {
    sickle()
        .args(["convert", "--to", "json"])
        .write_stdin("name = test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--from"));
}

#[test]
fn validate_valid_file() {
    sickle()
        .args(["validate", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn validate_quiet() {
    sickle()
        .args(["validate", "--quiet", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn fmt_outputs_canonical() {
    sickle()
        .args(["fmt", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("name = MyApp"));
}

#[test]
fn parse_default_output() {
    sickle()
        .args(["parse", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("[0]"))
        .stdout(predicate::str::contains("name"));
}

#[test]
fn parse_json_output() {
    sickle()
        .args(["parse", "--json", fixture("sample.ccl").to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"key\""))
        .stdout(predicate::str::contains("\"value\""));
}

#[test]
fn convert_toml_to_json() {
    sickle()
        .args(["convert", "--from", "toml", "--to", "json"])
        .write_stdin("[server]\nhost = \"localhost\"\nport = 8080")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"host\""))
        .stdout(predicate::str::contains("\"localhost\""));
}

#[test]
fn convert_comment_loss_skipped_with_yes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("with_comments.ccl");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "/= a comment").unwrap();
    writeln!(f, "name = Alice").unwrap();
    drop(f);

    sickle()
        .args(["convert", path.to_str().unwrap(), "--to", "json", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

#[test]
fn help_output() {
    sickle()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("convert"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("fmt"))
        .stdout(predicate::str::contains("parse"));
}

#[test]
fn round_trip_json_ccl_json() {
    let input = "{\"database\":{\"host\":\"db.example.com\",\"port\":\"5432\"}}";

    // JSON -> CCL
    let ccl_output = sickle()
        .args(["convert", "--from", "json", "--to", "ccl"])
        .write_stdin(input)
        .output()
        .expect("failed to run json->ccl");
    assert!(ccl_output.status.success());
    let ccl = String::from_utf8(ccl_output.stdout).unwrap();
    assert!(ccl.contains("database ="));
    assert!(ccl.contains("host = db.example.com"));

    // CCL -> JSON
    sickle()
        .args(["convert", "--from", "ccl", "--to", "json"])
        .write_stdin(ccl)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"host\""))
        .stdout(predicate::str::contains("\"db.example.com\""));
}

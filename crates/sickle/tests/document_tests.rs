//! Integration tests for the comment/format-preserving document API (issue #206).
//!
//! These exercise `load_document` / `Document::deserialize` / `Document::reserialize`,
//! verifying that hand-written comments, blank lines, key order, and formatting
//! survive a read -> mutate -> write cycle where data is unchanged, and that only
//! edited regions change.

#![cfg(feature = "document")]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Source {
    url: String,
    branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Config {
    name: String,
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    sources: BTreeMap<String, Source>,
}

const SAMPLE: &str = "\
/= top-level header comment
name = myapp
version = 1.0.0

tags =
  = alpha
  = beta
sources =
  github =
    url = https://github.com/x
    branch = main
";

#[test]
fn deserialize_matches_from_str() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    let cfg: Config = doc.deserialize().unwrap();
    assert_eq!(cfg.name, "myapp");
    assert_eq!(cfg.version, "1.0.0");
    assert_eq!(cfg.tags, vec!["alpha", "beta"]);
    assert_eq!(cfg.sources["github"].url, "https://github.com/x");
    assert_eq!(cfg.sources["github"].branch, "main");
}

#[test]
fn unchanged_roundtrip_is_byte_identical() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    let cfg: Config = doc.deserialize().unwrap();
    let out = doc.reserialize(&cfg).unwrap();
    assert_eq!(out, SAMPLE, "unchanged round-trip must be byte-identical");
}

#[test]
fn scalar_change_preserves_comments_and_blanks() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    cfg.version = "2.0.0".to_string();
    let out = doc.reserialize(&cfg).unwrap();

    assert!(
        out.contains("/= top-level header comment"),
        "comment kept: {out}"
    );
    assert!(out.contains("version = 2.0.0"), "value updated: {out}");
    assert!(!out.contains("1.0.0"), "old value gone: {out}");
    // Blank line between version and tags is preserved.
    assert!(
        out.contains("version = 2.0.0\n\ntags ="),
        "blank kept: {out}"
    );
    // Untouched nested block is preserved verbatim.
    assert!(out.contains("  github =\n    url = https://github.com/x\n    branch = main"));
}

#[test]
fn nested_add_preserves_existing_block_and_comments() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    cfg.sources.insert(
        "gitlab".to_string(),
        Source {
            url: "https://gitlab.com/y".to_string(),
            branch: "dev".to_string(),
        },
    );
    let out = doc.reserialize(&cfg).unwrap();

    assert!(out.contains("/= top-level header comment"));
    // existing github block untouched
    assert!(out.contains("  github =\n    url = https://github.com/x\n    branch = main"));
    // new gitlab block present and correctly nested
    assert!(
        out.contains("  gitlab =\n    url = https://gitlab.com/y\n    branch = dev"),
        "got: {out}"
    );
    // re-reading the result yields the edited config
    let doc2 = sickle::load_document(&out).unwrap();
    let cfg2: Config = doc2.deserialize().unwrap();
    assert_eq!(cfg2, cfg);
}

#[test]
fn list_push_appends_and_preserves() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    cfg.tags.push("gamma".to_string());
    let out = doc.reserialize(&cfg).unwrap();

    assert!(out.contains("  = alpha\n  = beta\n  = gamma"), "got: {out}");
    assert!(out.contains("/= top-level header comment"));
    let cfg2: Config = sickle::load_document(&out).unwrap().deserialize().unwrap();
    assert_eq!(cfg2.tags, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn removing_key_drops_its_attached_comment_but_keeps_header() {
    let input = "\
/= file header
name = myapp
version = 1.0.0
/= the nickname for the app
nickname = buddy
";
    let doc = sickle::load_document(input).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    assert_eq!(cfg.nickname.as_deref(), Some("buddy"));
    cfg.nickname = None;
    let out = doc.reserialize(&cfg).unwrap();

    assert!(out.contains("/= file header"), "header kept: {out}");
    assert!(!out.contains("nickname"), "removed key gone: {out}");
    assert!(
        !out.contains("the nickname for the app"),
        "attached comment dropped: {out}"
    );
}

#[test]
fn standalone_comment_block_survives_neighbor_removal() {
    let input = "\
name = myapp
version = 1.0.0

/= standalone divider

nickname = buddy
";
    let doc = sickle::load_document(input).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    cfg.nickname = None;
    let out = doc.reserialize(&cfg).unwrap();
    // A comment block separated from the removed key by a blank line is
    // standalone (not attached) and is kept.
    assert!(out.contains("standalone divider"), "standalone kept: {out}");
    assert!(!out.contains("nickname"));
}

#[test]
fn nested_comment_survives_sibling_add() {
    // The headline #206 use case: a comment INSIDE a nested block must survive
    // when a sibling is added to that block (recursive trivia preservation).
    let input = "\
name = myapp
version = 1.0.0
tags =
  = alpha
  = beta
sources =
  /= the canonical upstream remote
  github =
    url = https://github.com/x
    branch = main
";
    let doc = sickle::load_document(input).unwrap();
    let mut cfg: Config = doc.deserialize().unwrap();
    cfg.sources.insert(
        "gitlab".to_string(),
        Source {
            url: "https://gitlab.com/y".to_string(),
            branch: "dev".to_string(),
        },
    );
    let out = doc.reserialize(&cfg).unwrap();

    assert!(
        out.contains("/= the canonical upstream remote"),
        "nested comment preserved: {out}"
    );
    assert!(out.contains("  github =\n    url = https://github.com/x\n    branch = main"));
    assert!(out.contains("  gitlab =\n    url = https://gitlab.com/y\n    branch = dev"));
}

#[test]
fn unchanged_noncanonical_spacing_is_preserved() {
    // Value unchanged -> keep the user's exact (non-canonical) formatting.
    let input = "name=app\nversion =   1.0.0\n";
    let doc = sickle::load_document(input).unwrap();
    let cfg: Config = doc.deserialize().unwrap();
    let out = doc.reserialize(&cfg).unwrap();
    assert_eq!(out, input);

    // Changing a value re-renders just that entry canonically.
    let mut cfg2 = cfg.clone();
    cfg2.name = "newapp".to_string();
    let out2 = doc.reserialize(&cfg2).unwrap();
    assert!(out2.contains("name = newapp"), "got: {out2}");
    assert!(
        out2.contains("version =   1.0.0"),
        "untouched line kept: {out2}"
    );
}

#[test]
fn no_trailing_newline_is_preserved() {
    let input = "name = app\nversion = 1.0.0";
    let doc = sickle::load_document(input).unwrap();
    let cfg: Config = doc.deserialize().unwrap();
    let out = doc.reserialize(&cfg).unwrap();
    assert_eq!(out, input);
    assert!(!out.ends_with('\n'));
}

#[test]
fn display_renders_unmodified_source() {
    let doc = sickle::load_document(SAMPLE).unwrap();
    assert_eq!(doc.to_string(), SAMPLE);
    assert_eq!(doc.render(), SAMPLE);
}

//! Spec-compliance surface used by the CCL test suites.
//!
//! The CCL specification defines behavior at two levels: a flat `key = value`
//! lexing pass (`parse`, `parse_indented`, `print`, `filter`) and the
//! hierarchical model built from it. Sickle's public API exposes only the
//! hierarchical syntax tree, because the flat view is an implementation detail
//! of the lexer. The shared [ccl-test-data] suites, however, assert against the
//! flat view directly, so this module exposes it.
//!
//! **This module is not covered by semantic versioning.** It exists so the
//! conformance suites can be run against Sickle; application code should use
//! [`DocumentMut`](crate::DocumentMut).
//!
//! [ccl-test-data]: https://github.com/CatConfLang/ccl-test-data

use crate::error::ParseError;
use crate::lexer;
use crate::options::Options;

/// A flat `key = value` pair, before hierarchy is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatEntry {
    /// The key. Empty for bare list items; `/` for comment lines.
    pub key: String,
    /// The value, including any indented continuation lines.
    pub value: String,
}

impl FlatEntry {
    /// Create a flat entry.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Lex `input` into flat entries, preserving source order and duplicates.
pub fn flat_entries(input: &str, options: &Options) -> Result<Vec<FlatEntry>, ParseError> {
    Ok(lexer::flat_entries(input, options)
        .into_iter()
        .map(|e| FlatEntry::new(e.key, e.value))
        .collect())
}

/// Lex `input` into flat entries after stripping the block's shared indentation.
pub fn flat_entries_indented(input: &str, options: &Options) -> Result<Vec<FlatEntry>, ParseError> {
    Ok(lexer::flat_entries_indented(input, options)
        .into_iter()
        .map(|e| FlatEntry::new(e.key, e.value))
        .collect())
}

/// Render flat entries back to CCL text.
pub fn print_flat(entries: &[FlatEntry]) -> String {
    let internal: Vec<lexer::FlatEntry> = entries
        .iter()
        .map(|e| lexer::FlatEntry {
            key: e.key.clone(),
            value: e.value.clone(),
        })
        .collect();
    lexer::print_flat(&internal)
}

/// Drop comment entries, matching the specification's `filter` function.
pub fn filter_comments(entries: &[FlatEntry]) -> Vec<FlatEntry> {
    entries
        .iter()
        .filter(|e| !e.key.starts_with('/'))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_entries_expose_comment_keys() {
        let entries = flat_entries("/= note\na = 1", &Options::new()).unwrap();
        assert_eq!(entries[0], FlatEntry::new("/", "note"));
        assert_eq!(entries[1], FlatEntry::new("a", "1"));
    }

    #[test]
    fn filter_drops_comments() {
        let entries = flat_entries("/= note\na = 1", &Options::new()).unwrap();
        assert_eq!(filter_comments(&entries).len(), 1);
    }

    #[test]
    fn print_round_trips_flat_entries() {
        let source = "name = Alice\nconfig =\n  port = 8080";
        let entries = flat_entries(source, &Options::new()).unwrap();
        assert_eq!(print_flat(&entries), source);
    }

    #[test]
    fn indented_blocks_dedent_first() {
        let entries =
            flat_entries_indented("  servers = web1\n  cache = redis", &Options::new()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], FlatEntry::new("servers", "web1"));
    }
}

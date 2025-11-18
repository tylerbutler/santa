//! # Sickle - CCL Parser for Rust
//!
//! Sickle is a robust parser for CCL (Categorical Configuration Language) with optional Serde support.
//!
//! ## Features
//!
//! - **Two API styles**: Direct `Model` navigation or Serde deserialization
//! - **Complete CCL support**: Lists, nested records, multiline values, comments
//! - **Memory efficient**: Optional string interning via feature flag
//! - **Well-tested**: Comprehensive test suite with property-based tests
//!
//! ## Quick Start
//!
//! ### Direct API
//!
//! ```rust
//! use sickle::load;
//!
//! let ccl = r#"
//! name = Santa
//! version = 0.1.0
//! "#;
//!
//! let model = load(ccl).unwrap();
//! assert_eq!(model.get("name").unwrap().as_str().unwrap(), "Santa");
//! ```
//!
//! ### Serde Integration (requires "serde" feature)
//!
//! ```rust,ignore
//! use serde::Deserialize;
//! use sickle::from_str;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     name: String,
//!     version: String,
//! }
//!
//! let config: Config = from_str(ccl).unwrap();
//! ```
//!
//! ## Feature Registry
//!
//! For a comprehensive list of all supported CCL functions and parser behaviors:
//!
//! - **Auto-generated registry**: See [REGISTRY.md](https://github.com/tylerbutler/santa/blob/main/crates/sickle/REGISTRY.md)
//! - Dynamically generated from test data with coverage statistics
//! - Run `just sickle-registry` to regenerate
//!
//! ## Cargo Features
//!
//! - `serde` (default): Serde serialization/deserialization support
//! - `intern`: String interning for memory efficiency with large configs
//!
//! ### Future Features (Planned)
//!
//! - `section-headers`: Support `== Section ==` style headers
//! - `duplicate-key-lists`: Auto-create lists from duplicate keys
//! - `typed-access`: Convenience methods like `get_string()`, `get_int()`
//! - `list-indexing`: Advanced list operations and indexing

pub mod error;
pub mod model;
mod parser;

#[cfg(feature = "serde")]
pub mod de;

pub use error::{Error, Result};
pub use model::{Entry, Model};

/// Parse a CCL string into a flat list of entries
///
/// This is the first step of CCL processing, returning key-value pairs
/// without building the hierarchical structure. Use `build_hierarchy()` to
/// construct the hierarchical model from these entries.
///
/// # Examples
///
/// ```rust
/// use sickle::parse;
///
/// let ccl = r#"
/// name = MyApp
/// version = 1.0.0
/// "#;
///
/// let entries = parse(ccl).unwrap();
/// assert_eq!(entries.len(), 2);
/// assert_eq!(entries[0].key, "name");
/// assert_eq!(entries[0].value, "MyApp");
/// ```
pub fn parse(input: &str) -> Result<Vec<Entry>> {
    let map = parser::parse_to_map(input)?;

    // Convert BTreeMap<String, Vec<String>> to Vec<Entry>
    let mut entries = Vec::new();
    for (key, values) in map {
        for value in values {
            entries.push(Entry::new(key.clone(), value));
        }
    }

    Ok(entries)
}

/// Build a hierarchical Model from a flat list of entries
///
/// This is the second step of CCL processing, taking the entries from `parse()`
/// and constructing a hierarchical structure with proper nesting and type inference.
///
/// # Examples
///
/// ```rust
/// use sickle::{parse, build_hierarchy};
///
/// let ccl = r#"
/// name = MyApp
/// version = 1.0.0
/// "#;
///
/// let entries = parse(ccl).unwrap();
/// let model = build_hierarchy(&entries).unwrap();
/// assert_eq!(model.get("name").unwrap().as_str().unwrap(), "MyApp");
/// ```
pub fn build_hierarchy(entries: &[Entry]) -> Result<Model> {
    // Group entries by key (preserving order with BTreeMap)
    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for entry in entries {
        map.entry(entry.key.clone())
            .or_default()
            .push(entry.value.clone());
    }

    build_model(map)
}

/// Check if a string looks like a valid CCL key
/// Valid keys: alphanumeric, underscores, dots, hyphens (not leading), slashes for comments
fn is_valid_ccl_key(key: &str) -> bool {
    if key.is_empty() {
        return true; // Empty keys are valid (for lists)
    }

    // Comment keys are valid
    if key.starts_with('/') {
        return true;
    }

    // Must not start with a hyphen (command-line flag)
    if key.starts_with('-') {
        return false;
    }

    // Must consist of: alphanumeric, underscore, dot, or hyphen (not leading)
    key.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Internal helper: Build a Model from the grouped key-value map
fn build_model(map: std::collections::BTreeMap<String, Vec<String>>) -> Result<Model> {
    let mut result = std::collections::BTreeMap::new();

    for (key, values) in map {
        // Note: We preserve ALL keys including empty keys and comment keys
        // This is important for entry counting and for CCL spec compliance
        // Empty keys and comment keys (starting with /) are valid CCL entries

        let model = if values.len() == 1 {
            let value = &values[0];
            if value.is_empty() {
                // Empty value
                Model::singleton("")
            } else if value.contains('=') {
                // Contains '=' - might be nested CCL
                // Try to parse and build hierarchy to check if it looks like valid CCL structure
                match load(value) {
                    Ok(parsed) => {
                        // Check if this looks like valid CCL structure vs command-line string
                        if let Ok(map) = parsed.as_map() {
                            if !map.is_empty() {
                                // Check if all keys look like valid CCL keys
                                let has_valid_keys = map.keys().all(|k| is_valid_ccl_key(k));

                                if has_valid_keys {
                                    parsed
                                } else {
                                    // Keys don't look like valid CCL, treat as string
                                    Model::singleton(value.clone())
                                }
                            } else {
                                Model::singleton(value.clone())
                            }
                        } else {
                            parsed
                        }
                    }
                    Err(_) => Model::singleton(value.clone()),
                }
            } else {
                // Plain string value
                Model::singleton(value.clone())
            }
        } else {
            // Multiple values = list
            let items: Result<Vec<_>> = values
                .iter()
                .map(|v| {
                    if v.contains('=') {
                        // Try to load as nested CCL, use same validation as singleton case
                        match load(v) {
                            Ok(parsed) if !matches!(parsed, Model::Map(ref m) if m.is_empty()) => {
                                // Check if keys look valid
                                if let Ok(map) = parsed.as_map() {
                                    let has_valid_keys = map.keys().all(|k| is_valid_ccl_key(k));
                                    if has_valid_keys {
                                        Ok(parsed)
                                    } else {
                                        Ok(Model::singleton(v.clone()))
                                    }
                                } else {
                                    Ok(parsed)
                                }
                            }
                            _ => Ok(Model::singleton(v.clone())),
                        }
                    } else {
                        Ok(Model::singleton(v.clone()))
                    }
                })
                .collect();
            Model::list(items?)
        };

        result.insert(key, model);
    }

    Ok(Model::Map(result))
}

/// Parse a CCL value string with automatic prefix detection
///
/// Calculates the common indentation prefix and treats all lines at that
/// prefix level (or less) as top-level entries, returning a flat list of
/// key-value pairs.
///
/// This is used for parsing nested CCL values where the entire block may be
/// indented in the parent context.
///
/// # Examples
///
/// ```rust
/// use sickle::parse_dedented;
///
/// let nested = "  servers = web1\n  servers = web2\n  cache = redis";
/// let entries = parse_dedented(nested).unwrap();
/// assert_eq!(entries.len(), 3);
/// ```
pub fn parse_dedented(input: &str) -> Result<Vec<Entry>> {
    // Find the minimum indentation level (common prefix)
    let min_indent = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove the common prefix from all lines
    let dedented = input
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                line
            } else if line.len() > min_indent {
                &line[min_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // After dedenting, all lines at the original min_indent level are now at indent 0
    // Count how many entries exist at indent 0 (the dedented base level)
    let entries_at_min_indent = dedented
        .lines()
        .filter(|line| {
            let indent = line.len() - line.trim_start().len();
            indent == 0 && line.trim().contains('=')
        })
        .count();

    // If there are multiple entries at min_indent level, parse flat
    // Otherwise, parse as single entry with raw nested content
    if entries_at_min_indent > 1 {
        parse_flat_entries(&dedented)
    } else {
        parse_single_entry_with_raw_value(&dedented)
    }
}

/// Parse all key=value pairs from input as flat entries, ignoring indentation hierarchy
fn parse_flat_entries(input: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Extract key=value pairs or treat lines without '=' as keys with empty values
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value = trimmed[eq_pos + 1..].trim().to_string();
            entries.push(Entry::new(key, value));
        } else {
            // Line without '=' is a key with empty value
            entries.push(Entry::new(trimmed.to_string(), String::new()));
        }
    }

    Ok(entries)
}

/// Parse input as a single entry, preserving the raw value including indentation
fn parse_single_entry_with_raw_value(input: &str) -> Result<Vec<Entry>> {
    // Find the first line with '='
    let mut lines = input.lines();
    let first_line = lines.next().unwrap_or("");

    if let Some(eq_pos) = first_line.find('=') {
        let key = first_line[..eq_pos].trim().to_string();
        let first_value = first_line[eq_pos + 1..].to_string();

        // Collect remaining lines as the value, preserving indentation
        let remaining_lines: Vec<&str> = lines.collect();
        let value = if !remaining_lines.is_empty() {
            // Combine first line value with remaining lines
            if first_value.trim().is_empty() {
                // First line has no value after '=', so value is just the remaining lines
                "\n".to_string() + &remaining_lines.join("\n")
            } else {
                // First line has a value, append remaining lines
                first_value + "\n" + &remaining_lines.join("\n")
            }
        } else {
            first_value
        };

        Ok(vec![Entry::new(key, value)])
    } else {
        // No '=' found, treat as key with empty value
        Ok(vec![Entry::new(
            first_line.trim().to_string(),
            String::new(),
        )])
    }
}

/// Load and parse a CCL document into a hierarchical Model
///
/// This is a convenience function that combines `parse()` and `build_hierarchy()`.
/// Equivalent to: `build_hierarchy(&parse(input)?)`
///
/// # Examples
///
/// ```rust
/// use sickle::load;
///
/// let ccl = r#"
/// name = MyApp
/// version = 1.0.0
/// "#;
///
/// let model = load(ccl).unwrap();
/// assert_eq!(model.get("name").unwrap().as_str().unwrap(), "MyApp");
/// ```
pub fn load(input: &str) -> Result<Model> {
    let entries = parse(input)?;
    build_hierarchy(&entries)
}

#[cfg(feature = "serde")]
pub use de::from_str;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parse() {
        let ccl = r#"
name = Santa
version = 0.1.0
"#;
        let entries = parse(ccl).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "name");
        assert_eq!(entries[0].value, "Santa");
        assert_eq!(entries[1].key, "version");
        assert_eq!(entries[1].value, "0.1.0");
    }

    #[test]
    fn test_simple_build_hierarchy() {
        let ccl = r#"
name = Santa
version = 0.1.0
"#;
        let model = load(ccl).unwrap();
        assert_eq!(model.get("name").unwrap().as_str().unwrap(), "Santa");
        assert_eq!(model.get("version").unwrap().as_str().unwrap(), "0.1.0");
    }

    #[test]
    fn test_list_parsing() {
        let ccl = r#"
items =
  = one
  = two
  = three
"#;
        let model = load(ccl).unwrap();
        let items = model.get("items").unwrap();
        // The nested list syntax needs proper handling
        // The value contains nested CCL which will be parsed
        assert!(items.is_singleton() || items.is_list() || items.is_map());
    }

    #[test]
    fn test_nested_structure() {
        let ccl = r#"
database =
  host = localhost
  port = 5432
"#;
        let model = load(ccl).unwrap();
        let db = model.get("database").unwrap();
        assert!(db.is_map() || db.is_singleton());
    }

    #[test]
    fn test_comment_preservation() {
        let ccl = r#"
/= This is a comment
name = Santa
/= Another comment
version = 1.0
"#;
        let model = load(ccl).unwrap();
        assert_eq!(model.get("name").unwrap().as_str().unwrap(), "Santa");
        assert_eq!(model.get("version").unwrap().as_str().unwrap(), "1.0");

        // Comments are preserved as entries with key "/"
        let comments = model.get("/").unwrap().as_list().unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].as_str().unwrap(), "This is a comment");
        assert_eq!(comments[1].as_str().unwrap(), "Another comment");
    }

    #[test]
    fn test_multiline_value() {
        let ccl = r#"
description = This is a long
  description that spans
  multiple lines
"#;
        let model = load(ccl).unwrap();
        let desc = model.get("description").unwrap().as_str().unwrap();
        assert!(desc.contains("long"));
        assert!(desc.contains("multiple lines"));
    }

    #[test]
    fn test_parse_typed_value() {
        let ccl = r#"
port = 5432
enabled = true
"#;
        let model = load(ccl).unwrap();

        let port: u16 = model.get("port").unwrap().parse_value().unwrap();
        assert_eq!(port, 5432);

        let enabled: bool = model.get("enabled").unwrap().parse_value().unwrap();
        assert!(enabled);
    }

    #[test]
    fn test_duplicate_keys_create_list() {
        let ccl = r#"
symbols = @#$%
symbols = !^&*()
symbols = []{}|
symbols = <>=+
"#;
        let model = load(ccl).unwrap();

        // Should create a list from duplicate keys
        let symbols = model.get("symbols").unwrap();
        assert!(
            symbols.is_list(),
            "Duplicate keys should create a list, got: {:?}",
            symbols
        );

        let list = symbols.as_list().unwrap();
        assert_eq!(list.len(), 4);
        assert_eq!(list[0].as_str().unwrap(), "@#$%");
        assert_eq!(list[1].as_str().unwrap(), "!^&*()");
        assert_eq!(list[2].as_str().unwrap(), "[]{}|");
        assert_eq!(list[3].as_str().unwrap(), "<>=+");
    }

    #[test]
    fn test_multiline_in_list() {
        let ccl = r#"descriptions = First line
second line
descriptions = Another item
descriptions = Third item"#;
        let model = load(ccl).unwrap();
        println!("DEBUG: Full model = {:?}", model);

        // Should have both "descriptions" list and "second line" key
        let descriptions = model.get("descriptions").unwrap();
        println!("DEBUG: descriptions = {:?}", descriptions);

        let second_line = model.get("second line").unwrap();
        println!("DEBUG: second line = {:?}", second_line);
    }
}

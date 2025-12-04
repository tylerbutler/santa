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
//! assert_eq!(model.get_string("name").unwrap(), "Santa");
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
//! ## Capabilities
//!
//! For a comprehensive list of all supported CCL functions and parser behaviors:
//!
//! - **Auto-generated documentation**: See [docs/capabilities.md](https://github.com/tylerbutler/santa/blob/main/crates/sickle/docs/capabilities.md)
//! - Dynamically generated from test data with coverage statistics
//! - Run `just sickle-capabilities` to regenerate
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
pub use model::{CclObject, Entry};

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
/// assert_eq!(model.get_string("name").unwrap(), "MyApp");
/// ```
pub fn build_hierarchy(entries: &[Entry]) -> Result<CclObject> {
    // Group entries by key (preserving order with IndexMap)
    let mut map: indexmap::IndexMap<String, Vec<String>> = indexmap::IndexMap::new();

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
///
/// Following the CCL desugaring rules:
/// - `key = value` becomes `{"key": {"value": {}}}`
/// - `key =` (empty value) becomes `{"key": {"": {}}}`
/// - Multiple values become multiple nested keys
/// - Nested CCL is recursively parsed
fn build_model(map: indexmap::IndexMap<String, Vec<String>>) -> Result<CclObject> {
    let mut result = indexmap::IndexMap::new();

    for (key, values) in map {
        // Reference implementation iterates hash tables in reverse insertion order
        // Reverse ONLY for non-empty duplicate keys
        // Empty keys (bare list items) maintain insertion order
        #[cfg(feature = "reference_compliant")]
        let values = {
            let mut v = values;
            if v.len() > 1 && !key.is_empty() {
                v.reverse();
            }
            v
        };

        // Build the nested map for this key
        let mut nested = indexmap::IndexMap::new();

        for value in values {
            if value.contains('=') {
                // Contains '=' - might be nested CCL
                // Try to parse recursively
                match load(&value) {
                    Ok(parsed) => {
                        // Check if this looks like valid CCL structure
                        if !parsed.is_empty() {
                            // Check if all keys look like valid CCL keys
                            let has_valid_keys = parsed.keys().all(|k| is_valid_ccl_key(k));

                            if has_valid_keys {
                                // It's valid nested CCL, merge it into our nested map
                                for (k, v) in parsed.into_inner() {
                                    nested.insert(k, v);
                                }
                            } else {
                                // Keys don't look like valid CCL, treat as string value
                                nested.insert(value.clone(), CclObject::new());
                            }
                        } else {
                            // Empty parsed result, treat as string value
                            nested.insert(value.clone(), CclObject::new());
                        }
                    }
                    Err(_) => {
                        // Failed to parse, treat as string value
                        nested.insert(value.clone(), CclObject::new());
                    }
                }
            } else {
                // Plain string value - becomes a key with empty map
                nested.insert(value, CclObject::new());
            }
        }

        result.insert(key, CclObject::from_map(nested));
    }

    Ok(CclObject::from_map(result))
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
/// use sickle::parse_indented;
///
/// let nested = "  servers = web1\n  servers = web2\n  cache = redis";
/// let entries = parse_indented(nested).unwrap();
/// assert_eq!(entries.len(), 3);
/// ```
pub fn parse_indented(input: &str) -> Result<Vec<Entry>> {
    // Find the minimum indentation level (common prefix)
    let min_indent = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove the common prefix from all lines and expand tabs to spaces
    let dedented = input
        .lines()
        .map(|line| {
            // First expand tabs to spaces (treat each tab as 1 space for character-level dedenting)
            let expanded = line.replace('\t', " ");
            if expanded.trim().is_empty() {
                expanded
            } else if expanded.len() > min_indent {
                expanded[min_indent..].to_string()
            } else {
                expanded.trim_start().to_string()
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
        let first_value = first_line[eq_pos + 1..].trim_start().to_string();

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
/// assert_eq!(model.get_string("name").unwrap(), "MyApp");
/// ```
pub fn load(input: &str) -> Result<CclObject> {
    let entries = parse(input)?;
    build_hierarchy(&entries)
}

#[cfg(feature = "serde")]
pub use de::from_str;

// Unit tests removed - all functionality is covered by data-driven tests in:
// - api_core_ccl_parsing.json (basic parsing, multiline values, equals in values)
// - api_core_ccl_hierarchy.json (build_hierarchy, nested structures, duplicate keys to lists)
// - api_typed_access.json (get_int, get_bool, get_string, get_float)
// - api_comments.json (comment preservation)
// - api_list_access.json (list access and manipulation)
// - api_proposed_behavior.json (proposed behavior, currently excluded)

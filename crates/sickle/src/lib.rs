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
//! ```rust
//! use serde::Deserialize;
//! use sickle::from_str;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     name: String,
//!     version: String,
//! }
//!
//! let ccl = r#"
//! name = MyApp
//! version = 1.0.0
//! "#;
//!
//! let config: Config = from_str(ccl).unwrap();
//! assert_eq!(config.name, "MyApp");
//! assert_eq!(config.version, "1.0.0");
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
//! By default, sickle includes only the core types (`CclObject`, `Entry`, `Error`).
//! Enable features to add functionality:
//!
//! - `parse`: Core parsing (`parse`, `parse_indented`) - returns flat key-value entries
//! - `hierarchy`: Build hierarchical model (`build_hierarchy`, `load`) - includes `parse`
//! - `printer`: CCL printer for serializing back to canonical CCL text - includes `hierarchy`
//! - `serde-deserialize`: Serde deserialization (`from_str`) - includes `hierarchy`
//! - `serde-serialize`: Serde serialization (`to_string`) - includes `printer`
//! - `serde`: Both serialization and deserialization
//! - `intern`: String interning for memory efficiency with large configs
//! - `full`: Enable all features
//!
//! ### Future Features (Planned)
//!
//! - `section-headers`: Support `== Section ==` style headers
//! - `typed-access`: Convenience methods like `get_string()`, `get_int()`
//! - `list-indexing`: Advanced list operations and indexing

pub mod error;
pub mod model;
pub mod options;

#[cfg(feature = "parse")]
mod parser;

#[cfg(feature = "printer")]
pub mod printer;

#[cfg(feature = "serde-deserialize")]
pub mod de;

#[cfg(feature = "serde-serialize")]
pub mod ser;

pub use error::{Error, Result};
pub use model::{BoolOptions, CclObject, Entry, ListOptions};

// Re-export options types for crate-internal use
// ParserOptions is pub(crate) for now until API stabilizes
pub use options::{CrlfBehavior, ParserOptions, SpacingBehavior, TabBehavior};

#[cfg(feature = "printer")]
pub use printer::{CclPrinter, PrinterConfig};

/// Parse a CCL string into a flat list of entries
///
/// This is the first step of CCL processing, returning key-value pairs
/// without building the hierarchical structure. Use `build_hierarchy()` to
/// construct the hierarchical model from these entries.
///
/// Requires the `parse` feature.
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
#[cfg(feature = "parse")]
pub fn parse(input: &str) -> Result<Vec<Entry>> {
    parse_with_options_internal(input, &ParserOptions::default())
}

/// Parse a CCL string into a flat list of entries with custom options
///
/// This allows configuring parsing behavior such as:
/// - Spacing around `=` (strict vs loose)
/// - Tab handling (preserve vs convert to spaces)
/// - CRLF handling (preserve vs normalize to LF)
///
/// Requires the `parse` and `unstable` features.
///
/// **Note**: This API is unstable and may change. Use [`parse`] for stable API.
#[cfg(all(feature = "parse", feature = "unstable"))]
pub fn parse_with_options(input: &str, options: &ParserOptions) -> Result<Vec<Entry>> {
    parse_with_options_internal(input, options)
}

/// Internal implementation of parse_with_options
#[cfg(feature = "parse")]
fn parse_with_options_internal(input: &str, options: &ParserOptions) -> Result<Vec<Entry>> {
    let map = parser::parse_to_map(input, options)?;

    // Convert IndexMap<String, Vec<String>> to Vec<Entry>
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
/// Requires the `hierarchy` feature.
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
#[cfg(feature = "hierarchy")]
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

/// Check if a string looks like a valid CCL key for recursive parsing detection.
///
/// When a value contains `=`, we try parsing it as nested CCL. If the parser doesn't
/// find a valid ` = ` delimiter, the entire string becomes a single key. We detect this
/// by rejecting keys with spaces — real CCL keys from nested structures won't have spaces
/// since the ` = ` delimiter consumes them. Keys may contain special characters like
/// `/`, `\`, `:`, `@`, `#`, `[]`, `()` etc.
#[cfg(feature = "hierarchy")]
fn is_valid_ccl_key(key: &str) -> bool {
    if key.is_empty() {
        return true; // Empty keys are valid (for lists)
    }

    // Must not start with a hyphen (command-line flag)
    if key.starts_with('-') {
        return false;
    }

    // Keys with spaces are likely unparsed value strings, not real CCL keys.
    // When the parser doesn't find ` = `, the whole line becomes a "key" — reject those.
    !key.contains(' ')
}

/// Internal helper: Build a Model from the grouped key-value map
///
/// Following the CCL desugaring rules:
/// - `key = value` becomes `{"key": [{"value": [{}]}]}`
/// - `key =` (empty value) becomes `{"key": [{"": [{}]}]}`
/// - Multiple values become Vec entries for the same key
/// - Nested CCL is recursively parsed
#[cfg(feature = "hierarchy")]
fn build_model(map: indexmap::IndexMap<String, Vec<String>>) -> Result<CclObject> {
    let mut result = indexmap::IndexMap::new();

    for (key, values) in map {
        // Reference implementation iterates hash tables in lexical order
        // Sort ONLY for non-empty duplicate keys
        // Empty keys (bare list items) maintain insertion order
        #[cfg(feature = "reference_compliant")]
        let values = {
            let mut v = values;
            if v.len() > 1 && !key.is_empty() {
                v.sort();
            }
            v
        };

        // Build Vec of CclObjects for this key
        let mut nested_values = Vec::new();

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
                                // It's valid nested CCL, add it to our values
                                nested_values.push(parsed);
                            } else {
                                // Keys don't look like valid CCL, treat as string value
                                nested_values.push(CclObject::from_string(value));
                            }
                        } else {
                            // Empty parsed result, treat as string value
                            nested_values.push(CclObject::from_string(value));
                        }
                    }
                    Err(_) => {
                        // Failed to parse, treat as string value
                        nested_values.push(CclObject::from_string(value));
                    }
                }
            } else {
                // Plain string value
                nested_values.push(CclObject::from_string(value));
            }
        }

        // Handle duplicate keys according to CCL semantics:
        // - Empty keys (bare list items): always keep as Vec
        // - Non-empty keys with simple string values (like "item = first"): keep as list
        // - Non-empty keys with nested objects: compose them into a single object
        //
        // We determine if values are "simple strings" by checking if each has
        // exactly one key with an empty child (the pattern for string values)
        if !key.is_empty() && nested_values.len() > 1 {
            let all_simple_strings = nested_values.iter().all(|obj| {
                // A simple string value has exactly one key, and that key maps to empty
                obj.len() == 1 && obj.iter().next().is_some_and(|(_, v)| v.is_empty())
            });

            if all_simple_strings {
                // Keep as list - these are string values like "item = first"
                result.insert(key, nested_values);
            } else {
                // Compose into single object - these are nested structures
                let composed = nested_values
                    .iter()
                    .fold(CclObject::new(), |acc, obj| acc.compose(obj));
                result.insert(key, vec![composed]);
            }
        } else {
            result.insert(key, nested_values);
        }
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
/// Requires the `parse` feature.
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
#[cfg(feature = "parse")]
pub fn parse_indented(input: &str) -> Result<Vec<Entry>> {
    parse_indented_with_options_internal(input, &ParserOptions::default())
}

/// Parse a CCL value string with automatic prefix detection and custom options
///
/// Like [`parse_indented`], but allows configuring parsing behavior.
///
/// Requires the `parse` and `unstable` features.
///
/// **Note**: This API is unstable and may change. Use [`parse_indented`] for stable API.
#[cfg(all(feature = "parse", feature = "unstable"))]
pub fn parse_indented_with_options(input: &str, options: &ParserOptions) -> Result<Vec<Entry>> {
    parse_indented_with_options_internal(input, options)
}

/// Internal implementation of parse_indented_with_options
#[cfg(feature = "parse")]
fn parse_indented_with_options_internal(
    input: &str,
    options: &ParserOptions,
) -> Result<Vec<Entry>> {
    // Find the minimum indentation level (common prefix)
    let min_indent = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove the common prefix from all lines
    // Tab handling depends on options
    let dedented = input
        .lines()
        .map(|line| {
            // For dedenting calculation, we always need tabs converted to spaces
            // to properly calculate character positions
            let for_dedent = line.replace('\t', " ");

            if for_dedent.trim().is_empty() {
                // Empty/whitespace line: preserve original if preserving tabs, else use converted
                options.process_tabs(line).into_owned()
            } else if for_dedent.len() > min_indent {
                if options.preserve_tabs() {
                    // Preserve original line but remove min_indent chars
                    if line.len() > min_indent {
                        line[min_indent..].to_string()
                    } else {
                        line.trim_start().to_string()
                    }
                } else {
                    for_dedent[min_indent..].to_string()
                }
            } else if options.preserve_tabs() {
                line.trim_start().to_string()
            } else {
                for_dedent.trim_start().to_string()
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
        parse_flat_entries(&dedented, options)
    } else {
        parse_single_entry_with_raw_value(&dedented, options)
    }
}

/// Parse all key=value pairs from input as flat entries, ignoring indentation hierarchy
#[cfg(feature = "parse")]
fn parse_flat_entries(input: &str, options: &ParserOptions) -> Result<Vec<Entry>> {
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
            let value_raw = &trimmed[eq_pos + 1..];
            // Use options-aware trimming
            let value = if options.is_strict_spacing() {
                // Strict: trim only spaces
                value_raw
                    .trim_start_matches(' ')
                    .trim_end_matches(' ')
                    .to_string()
            } else {
                // Loose: trim all whitespace
                value_raw.trim().to_string()
            };
            entries.push(Entry::new(key, value));
        } else {
            // Line without '=' is a key with empty value
            entries.push(Entry::new(trimmed.to_string(), String::new()));
        }
    }

    Ok(entries)
}

/// Parse input as a single entry, preserving the raw value including indentation
#[cfg(feature = "parse")]
fn parse_single_entry_with_raw_value(input: &str, options: &ParserOptions) -> Result<Vec<Entry>> {
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
            let joined = if first_value.trim().is_empty() {
                // First line has no value after '=', so value is just the remaining lines
                "\n".to_string() + &remaining_lines.join("\n")
            } else {
                // First line has a value, append remaining lines
                first_value + "\n" + &remaining_lines.join("\n")
            };
            // Process tabs based on options
            options.process_tabs(&joined).into_owned()
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

/// Load and parse a CCL document into a hierarchical Model using default options
///
/// This is a convenience function that combines `parse()` and `build_hierarchy()`.
/// Equivalent to: `build_hierarchy(&parse(input)?)`
///
/// Requires the `hierarchy` feature.
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
#[cfg(feature = "hierarchy")]
pub fn load(input: &str) -> Result<CclObject> {
    load_with_options_internal(input, &ParserOptions::default())
}

/// Load and parse a CCL document with custom options
///
/// This is a convenience function that combines `parse_with_options()` and `build_hierarchy()`.
///
/// Requires the `hierarchy` and `unstable` features.
///
/// **Note**: This API is unstable and may change. Use [`load`] for stable API.
#[cfg(all(feature = "hierarchy", feature = "unstable"))]
pub fn load_with_options(input: &str, options: &ParserOptions) -> Result<CclObject> {
    load_with_options_internal(input, options)
}

/// Internal implementation of load_with_options
#[cfg(feature = "hierarchy")]
fn load_with_options_internal(input: &str, options: &ParserOptions) -> Result<CclObject> {
    let entries = parse_with_options_internal(input, options)?;
    build_hierarchy(&entries)
}

#[cfg(feature = "serde-deserialize")]
pub use de::{from_str, from_str_with_options};

#[cfg(feature = "serde-serialize")]
pub use ser::to_string;

// Unit tests removed - all functionality is covered by data-driven tests in:
// - api_core_ccl_parsing.json (basic parsing, multiline values, equals in values)
// - api_core_ccl_hierarchy.json (build_hierarchy, nested structures, duplicate keys to lists)
// - api_typed_access.json (get_int, get_bool, get_string, get_float)
// - api_comments.json (comment preservation)
// - api_list_access.json (list access and manipulation)
// - api_proposed_behavior.json (proposed behavior, currently excluded)

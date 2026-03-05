//! CCL Printer - serialize CclObject back to canonical CCL text format
//!
//! This module provides functionality to convert a parsed CCL structure back into
//! its canonical text representation. The printer produces well-formatted CCL output
//! that can be parsed back into an equivalent structure.
//!
//! ## Canonical Format Rules
//!
//! 1. **Indentation**: 2 spaces per nesting level
//! 2. **Key-Value Pairs**: `key = value` on single lines
//! 3. **Nested Content**: Indented under parent key with empty inline value
//! 4. **Comments**: Preserved as `/= comment text`
//! 5. **Lists**: Output format depends on configuration
//!
//! ## Example
//!
//! ```rust
//! use sickle::{load, printer::CclPrinter};
//!
//! let ccl = r#"
//! name = MyApp
//! server =
//!   host = localhost
//!   port = 8080
//! "#;
//!
//! let model = load(ccl).unwrap();
//! let printer = CclPrinter::new();
//! let output = printer.print(&model);
//! println!("{}", output);
//! ```

use crate::model::CclObject;
use crate::Entry;

/// Print a list of CCL entries back to canonical CCL text format.
///
/// This is the entry-level `print` function from the CCL specification.
/// It converts flat key-value entries (from `parse()`) back into CCL text,
/// preserving the original structure.
///
/// ## Canonical Format
///
/// Each entry is formatted as `{key} = {value}`, separated by newlines.
/// - Empty keys produce ` = {value}` (space before `=`)
/// - Empty values produce `{key} = ` (trailing space after `=`)
/// - Multiline values (starting with `\n`) produce `{key} = \n  ...`
///
/// ## Round-Trip Property
///
/// For inputs in standard format:
/// ```text
/// parse(print(parse(x))) == parse(x)
/// ```
///
/// ## Example
///
/// ```rust
/// use sickle::{parse, printer::print};
///
/// let ccl = "name = Alice\nconfig =\n  port = 8080";
/// let entries = parse(ccl).unwrap();
/// let output = print(&entries);
/// assert_eq!(output, "name = Alice\nconfig = \n  port = 8080");
/// ```
pub fn print(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|entry| {
            if entry.key.is_empty() {
                // Empty keys use bare list syntax: `= value` (no leading space)
                format!("= {}", entry.value)
            } else {
                format!("{} = {}", entry.key, entry.value)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Check whether `print(parse(input))` round-trips correctly.
///
/// Verifies the weaker round-trip property:
/// ```text
/// parse(print(parse(x))) == parse(x)
/// ```
///
/// This means printing and re-parsing produces the same entries as the
/// original parse, even if the printed text differs from the original input
/// (e.g., due to whitespace normalization).
///
/// Requires the `parse` feature (which is implied by `printer`).
///
/// ## Example
///
/// ```rust
/// use sickle::printer::round_trip;
///
/// assert!(round_trip("name = Alice").unwrap());
/// assert!(round_trip("config =\n  port = 8080").unwrap());
/// ```
pub fn round_trip(input: &str) -> crate::Result<bool> {
    let entries1 = crate::parse(input)?;
    let printed = print(&entries1);
    let entries2 = crate::parse(&printed)?;
    Ok(entries1 == entries2)
}

/// Configuration options for CCL printing
#[derive(Debug, Clone)]
pub struct PrinterConfig {
    /// Number of spaces per indentation level (default: 2)
    pub indent_size: usize,
    /// Use bare list syntax (`= item`) instead of duplicate keys (`key = item`)
    /// when printing list-like structures (default: true)
    pub use_bare_list_syntax: bool,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            use_bare_list_syntax: true,
        }
    }
}

impl PrinterConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation size
    pub fn with_indent_size(mut self, size: usize) -> Self {
        self.indent_size = size;
        self
    }

    /// Set whether to use bare list syntax for lists
    pub fn with_bare_list_syntax(mut self, use_bare: bool) -> Self {
        self.use_bare_list_syntax = use_bare;
        self
    }
}

/// CCL Printer for serializing CclObject back to canonical CCL text
#[derive(Debug, Clone)]
pub struct CclPrinter {
    config: PrinterConfig,
}

impl Default for CclPrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl CclPrinter {
    /// Create a new printer with default configuration
    pub fn new() -> Self {
        Self {
            config: PrinterConfig::default(),
        }
    }

    /// Create a new printer with custom configuration
    pub fn with_config(config: PrinterConfig) -> Self {
        Self { config }
    }

    /// Print a CclObject to its canonical CCL text representation
    pub fn print(&self, model: &CclObject) -> String {
        let mut output = String::new();
        self.print_object(model, 0, &mut output);
        // Remove trailing newline if present for cleaner output
        output.trim_end_matches('\n').to_string()
    }

    /// Print a CclObject at the given indentation level
    fn print_object(&self, model: &CclObject, indent: usize, output: &mut String) {
        let indent_str = " ".repeat(indent);

        // Use iter_all() to print all values including duplicates
        for (key, value) in model.iter_all() {
            self.print_entry(key, value, &indent_str, indent, output);
        }
    }

    /// Print a single key-value entry
    fn print_entry(
        &self,
        key: &str,
        value: &CclObject,
        indent_str: &str,
        indent: usize,
        output: &mut String,
    ) {
        // Handle blank lines: empty key with empty value
        if key.is_empty() && value.is_empty() {
            output.push('\n');
            return;
        }

        // Handle comment lines: key starts with "/=" and has empty value
        if key.starts_with("/=") && value.is_empty() {
            output.push_str(indent_str);
            output.push_str(key);
            output.push('\n');
            return;
        }

        if value.is_empty() {
            // Leaf value: key with empty map represents a string value
            // In CCL, the key itself IS the value at this level
            // This case shouldn't normally be reached at top level
            // But for nested values, this represents the string encoding
            output.push_str(indent_str);
            output.push_str(key);
            output.push_str(" =\n");
        } else if self.is_string_value(value) {
            // String value: single child with empty map
            // The string value is stored as {value_string: {}}
            let string_value = value.keys().next().unwrap();
            output.push_str(indent_str);
            if key.is_empty() {
                // Bare list syntax: = value (no leading space)
                output.push_str("= ");
            } else {
                // Normal key-value: key = value
                output.push_str(key);
                output.push_str(" = ");
            }
            output.push_str(string_value);
            output.push('\n');
        } else if self.is_list_value(value) {
            // List value: multiple children, all with empty maps
            self.print_list(key, value, indent_str, indent, output);
        } else {
            // Nested object: key with children that have their own structure
            output.push_str(indent_str);
            output.push_str(key);
            output.push_str(" =\n");
            self.print_object(value, indent + self.config.indent_size, output);
        }
    }

    /// Check if a CclObject represents a simple string value
    /// A string value is {string: {}} - single key with empty map
    fn is_string_value(&self, value: &CclObject) -> bool {
        if value.len() != 1 {
            return false;
        }
        if let Some((_, child)) = value.iter().next() {
            return child.is_empty();
        }
        false
    }

    /// Check if a CclObject represents a list value
    /// A list is multiple keys where all children are empty maps (terminal values)
    fn is_list_value(&self, value: &CclObject) -> bool {
        if value.len() < 2 {
            return false;
        }
        // All children must be terminal (empty maps) for it to be a list
        value.values().all(|child| child.is_empty())
    }

    /// Print a list value
    fn print_list(
        &self,
        key: &str,
        value: &CclObject,
        indent_str: &str,
        indent: usize,
        output: &mut String,
    ) {
        let child_indent_str = " ".repeat(indent + self.config.indent_size);

        // Check if this is a bare list (key is empty or we should use bare syntax)
        if key.is_empty() || (self.config.use_bare_list_syntax && !key.is_empty()) {
            // Bare list syntax: parent key, then indented `= item` entries
            if !key.is_empty() {
                output.push_str(indent_str);
                output.push_str(key);
                output.push_str(" =\n");
            }
            for item_key in value.keys() {
                if key.is_empty() {
                    // Already at the list level, use current indent
                    output.push_str(indent_str);
                } else {
                    output.push_str(&child_indent_str);
                }
                output.push_str("= ");
                output.push_str(item_key);
                output.push('\n');
            }
        } else {
            // Duplicate keys syntax: repeat `key = item` for each value
            for item_key in value.keys() {
                output.push_str(indent_str);
                output.push_str(key);
                output.push_str(" = ");
                output.push_str(item_key);
                output.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load;

    // ========================================================================
    // Entry-level print() tests (from ccl-test-data property_round_trip.json)
    // ========================================================================

    #[test]
    fn test_print_basic() {
        let input = "key = value\nnested =\n  sub = val";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(output, "key = value\nnested = \n  sub = val");
    }

    #[test]
    fn test_print_empty_keys_lists() {
        let input = "= item1\n= item2\nregular = value";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(output, "= item1\n= item2\nregular = value");
    }

    #[test]
    fn test_print_nested_structures() {
        let input = "config =\n  host = localhost\n  port = 8080\n  db =\n    name = mydb\n    user = admin";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(
            output,
            "config = \n  host = localhost\n  port = 8080\n  db =\n    name = mydb\n    user = admin"
        );
    }

    #[test]
    fn test_print_multiline_values() {
        let input = "script =\n  #!/bin/bash\n  echo hello\n  exit 0";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(output, "script = \n  #!/bin/bash\n  echo hello\n  exit 0");
    }

    #[test]
    fn test_print_empty_value() {
        let input = "empty_section =\n\nother = value";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(output, "empty_section = \nother = value");
    }

    #[test]
    fn test_print_deeply_nested() {
        let input =
            "level1 =\n  level2 =\n    level3 =\n      level4 =\n        deep = value\n        = deep_item";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(
            output,
            "level1 = \n  level2 =\n    level3 =\n      level4 =\n        deep = value\n        = deep_item"
        );
    }

    #[test]
    fn test_print_mixed_content() {
        // Entry preservation: duplicate empty-key entries maintain original interleaving
        let input =
            "name = Alice\n= first item\nconfig =\n  port = 3000\n= second item\nfinal = value";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(
            output,
            "name = Alice\n= first item\nconfig = \n  port = 3000\n= second item\nfinal = value"
        );
    }

    #[test]
    fn test_print_complex_nesting() {
        let input = "app =\n  = item1\n  config =\n    = nested_item\n    db =\n      host = localhost\n      = db_item\n  = item2";
        let entries = crate::parse(input).unwrap();
        let output = print(&entries);
        assert_eq!(
            output,
            "app = \n  = item1\n  config =\n    = nested_item\n    db =\n      host = localhost\n      = db_item\n  = item2"
        );
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    #[test]
    fn test_round_trip_basic() {
        assert!(round_trip("key = value\nnested =\n  sub = val").unwrap());
    }

    #[test]
    fn test_round_trip_empty_keys() {
        assert!(round_trip("= item1\n= item2\nregular = value").unwrap());
    }

    #[test]
    fn test_round_trip_nested() {
        assert!(round_trip("config =\n  host = localhost\n  port = 8080\n  db =\n    name = mydb\n    user = admin").unwrap());
    }

    #[test]
    fn test_round_trip_multiline() {
        assert!(round_trip("script =\n  #!/bin/bash\n  echo hello\n  exit 0").unwrap());
    }

    #[test]
    fn test_round_trip_deeply_nested() {
        assert!(round_trip("level1 =\n  level2 =\n    level3 =\n      level4 =\n        deep = value\n        = deep_item").unwrap());
    }

    #[test]
    fn test_round_trip_mixed_content() {
        // print() preserves interleaved entry order thanks to entry preservation.
        // Empty keys use `= value` format (no leading space), which correctly
        // re-parses as a new entry at base indentation level.
        let input =
            "name = Alice\n= first item\nconfig =\n  port = 3000\n= second item\nfinal = value";
        let entries = crate::parse(input).unwrap();
        let printed = print(&entries);
        // Verify print preserves interleaved order
        assert_eq!(
            printed,
            "name = Alice\n= first item\nconfig = \n  port = 3000\n= second item\nfinal = value"
        );
        // Round-trip now works correctly since `= item` at indent 0
        // re-parses as a new entry (not a continuation of the previous one).
        assert!(round_trip(input).unwrap());
    }

    #[test]
    fn test_round_trip_empty_value() {
        assert!(round_trip("empty_section =\n\nother = value").unwrap());
    }

    // ========================================================================
    // CclPrinter (model-level) tests
    // ========================================================================

    #[test]
    fn test_simple_key_value() {
        let ccl = "name = Alice\nage = 42";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert_eq!(output, "name = Alice\nage = 42");
    }

    #[test]
    fn test_nested_object() {
        let ccl = "server =\n  host = localhost\n  port = 8080";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert_eq!(output, "server =\n  host = localhost\n  port = 8080");
    }

    #[test]
    fn test_list_with_bare_syntax() {
        let ccl = "servers =\n  = web1\n  = web2";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        // The internal representation uses empty key for bare lists
        // Output should reconstruct bare list syntax
        assert!(output.contains("servers ="));
        assert!(output.contains("= web1") || output.contains("= web2"));
    }

    #[test]
    fn test_list_with_duplicate_keys_syntax() {
        let ccl = "servers = web1\nservers = web2\nservers = web3";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::with_config(PrinterConfig::new().with_bare_list_syntax(false));
        let output = printer.print(&model);
        // With duplicate key syntax, should produce repeated key entries
        assert!(output.contains("servers = "));
    }

    #[test]
    fn test_comment_preservation() {
        let ccl = "/= This is a comment\nname = Alice";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert!(output.contains("/"));
    }

    #[test]
    fn test_deeply_nested() {
        let ccl = "level1 =\n  level2 =\n    level3 = value";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert!(output.contains("level1 ="));
        assert!(output.contains("  level2 ="));
        assert!(output.contains("    level3 = value"));
    }

    #[test]
    fn test_custom_indent_size() {
        let ccl = "parent =\n  child = value";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::with_config(PrinterConfig::new().with_indent_size(4));
        let output = printer.print(&model);
        assert!(output.contains("parent =\n    child = value"));
    }

    #[test]
    fn test_empty_model() {
        let ccl = "";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert!(output.is_empty());
    }

    #[test]
    fn test_mixed_nested_and_lists() {
        let ccl = "config =\n  server = web1\n  server = web2\n  port = 80";
        let model = load(ccl).unwrap();
        let printer = CclPrinter::new();
        let output = printer.print(&model);
        assert!(output.contains("config ="));
        assert!(output.contains("port = 80"));
    }

    #[test]
    fn test_from_list_uses_correct_indentation() {
        // Test that CclObject::from_list() produces correct 2-space indentation
        // This is the code path used by generate_index.rs
        use crate::CclObject;

        let mut model = CclObject::new();
        let map = model.inner_mut();
        map.insert(
            "package".to_string(),
            vec![CclObject::from_list(vec!["brew", "scoop", "nix"])],
        );

        let printer = CclPrinter::new();
        let output = printer.print(&model);

        // Should produce:
        // package =
        //   = brew
        //   = scoop
        //   = nix
        assert!(output.contains("package =\n"));
        assert!(output.contains("  = brew\n")); // Exactly 2 spaces
        assert!(output.contains("  = scoop\n"));
        assert!(output.contains("  = nix")); // Last line has no trailing newline

        // Verify NO 3-space indentation (the bug we fixed)
        assert!(!output.contains("   = "));
    }
}

//! Core CCL parser implementation
//!
//! This module implements the CCL parsing algorithm as described in the specification:
//! 1. Parse text into flat key-value entries
//! 2. Build hierarchy through recursive processing

use crate::error::Result;
use indexmap::IndexMap;

/// A parsed CCL entry (key-value pair)
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    key: String,
    value: String,
    indent: usize,
}

/// Parse CCL text into a flat list of entries
///
/// This respects indentation - lines at the base level start new entries,
/// lines indented further become part of the current entry's value
fn parse_entries(input: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current_key: Option<(String, usize)> = None;
    let mut value_lines: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;

    for line in input.lines() {
        // Count leading spaces
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            if current_key.is_some() {
                value_lines.push(String::new());
            }
            continue;
        }

        // Determine base indentation from first non-empty line
        if base_indent.is_none() {
            base_indent = Some(indent);
        }

        let base = base_indent.unwrap_or(0);

        // Check if this line starts a new entry at the base level
        if indent <= base && trimmed.contains('=') {
            // Save previous entry if exists
            if let Some((key, key_indent)) = current_key.take() {
                let value = value_lines.join("\n").trim_end().to_string();
                entries.push(Entry {
                    key,
                    value,
                    indent: key_indent,
                });
                value_lines.clear();
            }

            // Parse new key-value pair
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();

                current_key = Some((key, indent));
                if !value.is_empty() {
                    value_lines.push(value);
                } else {
                    // Empty value after '=' - add empty string to create leading newline
                    // when continuation lines are added
                    value_lines.push(String::new());
                }
            }
        } else if let Some((_, key_indent)) = current_key {
            // Only treat as continuation if indented MORE than the key line
            if indent > key_indent {
                // This line is indented relative to the key - it's part of the current value
                // Preserve the full line for nested structures
                value_lines.push(line.to_string());
            } else {
                // Not indented more than key - save current entry and start new one
                let (key, key_indent_final) = current_key.take().unwrap();
                let value = value_lines.join("\n").trim_end().to_string();
                entries.push(Entry {
                    key,
                    value,
                    indent: key_indent_final,
                });
                value_lines.clear();

                // This line becomes a new key with empty value (no '=' sign)
                current_key = Some((trimmed.to_string(), indent));
            }
        }
    }

    // Don't forget the last entry
    if let Some((key, key_indent)) = current_key {
        let value = value_lines.join("\n").trim_end().to_string();
        entries.push(Entry {
            key,
            value,
            indent: key_indent,
        });
    }

    entries
}

/// Remove common leading whitespace from all lines while preserving relative indentation
///
/// Note: This function is currently unused as the CCL specification requires preserving
/// indentation as-is. Kept for potential future use with specific parser behaviors.
#[allow(dead_code)]
fn dedent(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    // Find the minimum indentation (ignoring empty lines)
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove that amount of leading whitespace from each line
    lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                ""
            } else if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Build hierarchical structure from flat entries
pub(crate) fn parse_to_map(input: &str) -> Result<IndexMap<String, Vec<String>>> {
    let entries = parse_entries(input);
    let mut result: IndexMap<String, Vec<String>> = IndexMap::new();

    for entry in entries {
        // Preserve indentation as-is per CCL specification
        result.entry(entry.key).or_default().push(entry.value);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key_value() {
        let input = "name = value";
        let entries = parse_entries(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "name");
        assert_eq!(entries[0].value, "value");
    }

    #[test]
    fn test_multiple_entries() {
        let input = r#"
name = Santa
version = 0.1.0
author = Tyler
"#;
        let entries = parse_entries(input);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].key, "name");
        assert_eq!(entries[0].value, "Santa");
        assert_eq!(entries[1].key, "version");
        assert_eq!(entries[1].value, "0.1.0");
    }

    #[test]
    fn test_empty_key_list() {
        let input = r#"
= item1
= item2
= item3
"#;
        let entries = parse_entries(input);
        assert_eq!(entries.len(), 3);
        assert!(entries[0].key.is_empty());
        assert_eq!(entries[0].value, "item1");
    }

    #[test]
    fn test_multiline_value() {
        let input = r#"
description = This is a
  multi-line
  value
"#;
        let entries = parse_entries(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "description");
        assert!(entries[0].value.contains("multi-line"));
    }

    #[test]
    fn test_value_with_equals() {
        let input = "command = npm list --depth=0";
        let entries = parse_entries(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "command");
        assert_eq!(entries[0].value, "npm list --depth=0");
    }

    #[test]
    fn test_nested_structure() {
        let input = r#"
database =
  host = localhost
  port = 5432
"#;
        let map = parse_to_map(input).unwrap();
        assert!(map.contains_key("database"));
    }

    #[test]
    fn test_unindented_line_not_continuation() {
        // Per CCL spec: "Lines indented more than previous = part of that value"
        // This means unindented lines should NOT be continuations
        let input = r#"descriptions = First line
second line
descriptions = Another item"#;

        let map = parse_to_map(input).unwrap();

        // "second line" should be a separate key, not part of "First line"
        assert!(map.contains_key("second line"));
        assert_eq!(map.get("second line").unwrap()[0], "");

        // "descriptions" should have exactly 2 items
        let descriptions = map.get("descriptions").unwrap();
        assert_eq!(descriptions.len(), 2);
        assert_eq!(descriptions[0], "First line");
        assert_eq!(descriptions[1], "Another item");
    }

    #[test]
    fn test_indented_line_is_continuation() {
        // Properly indented lines should be part of the value
        let input = r#"descriptions = First line
  second line
descriptions = Another item"#;

        let map = parse_to_map(input).unwrap();

        // "second line" should NOT be a separate key
        assert!(!map.contains_key("second line"));

        // First description should contain both lines
        let descriptions = map.get("descriptions").unwrap();
        assert_eq!(descriptions.len(), 2);
        assert_eq!(descriptions[0], "First line\n  second line");
        assert_eq!(descriptions[1], "Another item");
    }

    #[test]
    fn test_mixed_indentation_levels() {
        let input = r#"key1 = value1
  indented continuation
key2 = value2
not indented key
  indented for not indented"#;

        let map = parse_to_map(input).unwrap();

        // key1 should have continuation
        assert_eq!(
            map.get("key1").unwrap()[0],
            "value1\n  indented continuation"
        );

        // key2 should NOT have continuation
        assert_eq!(map.get("key2").unwrap()[0], "value2");

        // "not indented key" should be separate
        assert!(map.contains_key("not indented key"));

        // And it should have its own continuation (with indentation preserved per CCL spec)
        assert_eq!(
            map.get("not indented key").unwrap()[0],
            "  indented for not indented"
        );
    }
}

//! Core CCL parser implementation
//!
//! This module implements the CCL parsing algorithm as described in the specification:
//! 1. Parse text into flat key-value entries
//! 2. Build hierarchy through recursive processing

use crate::error::Result;
use crate::options::ParserOptions;
use indexmap::IndexMap;

/// A parsed CCL entry (key-value pair)
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    key: String,
    value: String,
    indent: usize,
}

/// Normalize input by handling multiline keys (newlines before '=')
/// This joins lines where a key spans multiple lines before the equals sign
///
/// Strategy: Only join lines if they form a multiline key at the SAME indentation level.
/// Indented lines are continuation values, not multiline keys.
///
/// Important: A line without '=' following a complete `key = value` line should NOT
/// be joined with subsequent lines. It should be treated as a standalone key with
/// empty value.
fn normalize_multiline_keys(input: &str, preserve_cr: bool) -> String {
    // Use split('\n') to preserve \r when needed, otherwise use lines() for standard behavior
    let lines: Vec<&str> = if preserve_cr {
        input.split('\n').collect()
    } else {
        input.lines().collect()
    };
    let mut result = String::new();
    let mut i = 0;
    let mut base_indent: Option<usize> = None;
    let mut prev_had_complete_entry = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let line_indent = line.len() - line.trim_start().len();

        // Skip leading empty lines
        if trimmed.is_empty() && result.is_empty() {
            i += 1;
            continue;
        }

        // Determine base indentation from first non-empty line
        if base_indent.is_none() && !trimmed.is_empty() {
            base_indent = Some(line_indent);
        }

        let base = base_indent.unwrap_or(0);

        // Track if this line is a complete key=value entry
        let is_complete_entry = trimmed.contains('=');

        // If this line is indented more than base level, it's a continuation value
        // Pass it through unchanged - don't try to interpret it as a multiline key
        if line_indent > base {
            result.push_str(line);
            result.push('\n');
            i += 1;
            continue;
        }

        // Check if this is a multiline key pattern:
        // Current line has no '=' AND is at or below base level
        // AND the previous line was NOT a complete entry (otherwise this is a standalone key)
        if !trimmed.is_empty() && !trimmed.contains('=') && !prev_had_complete_entry {
            // Look ahead for the next line with '=' at SAME OR LESS indentation
            let mut j = i + 1;
            let mut found_equals = false;
            while j < lines.len() {
                let next_line = lines[j];
                let next_trimmed = next_line.trim();
                let next_indent = next_line.len() - next_line.trim_start().len();

                if next_trimmed.is_empty() {
                    j += 1;
                    continue;
                }

                // If indented MORE than current line, this is a continuation value, not a key
                if next_indent > line_indent {
                    break;
                }

                if next_trimmed.contains('=') {
                    found_equals = true;
                    break;
                }

                // Another line at same/less indentation without '=' - could be multiline key continuation
                j += 1;
            }

            if found_equals && j < lines.len() {
                // Join lines from i to j into a single key=value line
                // Only join lines at the same indentation level
                let mut key_parts = vec![trimmed];
                for part_line in lines.iter().take(j).skip(i + 1) {
                    let part_trimmed = part_line.trim();
                    let part_indent = part_line.len() - part_line.trim_start().len();

                    if !part_trimmed.is_empty() && part_indent <= line_indent {
                        key_parts.push(part_trimmed);
                    }
                }
                let joined_key = key_parts.join(" ");
                result.push_str(&joined_key);
                result.push_str(lines[j].trim());
                result.push('\n');
                i = j + 1;
                prev_had_complete_entry = true; // The joined result is a complete entry
                continue;
            }
        }

        // Normal line - pass through
        result.push_str(line);
        result.push('\n');
        prev_had_complete_entry = is_complete_entry;
        i += 1;
    }

    result
}

/// Trim only spaces (not tabs) from the start of a string
fn trim_spaces_start(s: &str) -> &str {
    s.trim_start_matches(' ')
}

/// Trim whitespace from a string, optionally preserving CR
/// When preserve_cr is true, only trims spaces and tabs (not \r)
fn trim_with_cr_option(s: &str, preserve_cr: bool) -> &str {
    if preserve_cr {
        // Only trim spaces and tabs, preserving \r
        s.trim_matches([' ', '\t'])
    } else {
        s.trim()
    }
}

/// Trim leading whitespace, optionally preserving CR
fn trim_start_with_cr_option(s: &str, preserve_cr: bool) -> &str {
    if preserve_cr {
        s.trim_start_matches([' ', '\t'])
    } else {
        s.trim_start()
    }
}

/// Find the position of a valid `=` delimiter based on spacing options
///
/// - Strict spacing: requires ` = ` (space-equals-space), or ` =` at end of line
/// - Loose spacing: any `=` is valid
///
/// Returns the byte position of `=` if found, or None if no valid delimiter exists.
fn find_delimiter(s: &str, options: &ParserOptions) -> Option<usize> {
    if options.is_strict_spacing() {
        // Strict spacing: require ` = ` pattern (space before and after equals)
        // OR ` =` at the end of the string (for empty values like "key =")
        if let Some(pos) = s.find(" = ") {
            return Some(pos + 1);
        }
        // Check for ` =` at end of string (space before equals, nothing after)
        if s.ends_with(" =") {
            return Some(s.len() - 1);
        }
        None
    } else {
        // Loose spacing: any `=` is a valid delimiter
        s.find('=')
    }
}

/// Trim leading whitespace from value on the same line as the key
///
/// The behavior depends on tab options:
/// - With tabs_preserve: only trim spaces, tabs are value content
/// - With tabs_to_spaces: trim all leading whitespace (tabs are delimiter whitespace)
///
/// Examples:
/// - `key = value` → value = `value` (space trimmed)
/// - `key = \tvalue` with tabs_preserve → value = `\tvalue` (tab is content)
/// - `key\t=\tdata` with tabs_to_spaces → value = `data` (tab trimmed)
fn trim_value(s: &str, options: &ParserOptions) -> String {
    if options.preserve_tabs() {
        // Only trim spaces, tabs are value content
        trim_spaces_start(s).to_string()
    } else {
        // Trim all whitespace - tabs are delimiter whitespace (converted later)
        s.trim_start().to_string()
    }
}

/// Parse CCL text into a flat list of entries
///
/// This respects indentation - lines at the base level start new entries,
/// lines indented further become part of the current entry's value
fn parse_entries(input: &str, options: &ParserOptions) -> Vec<Entry> {
    // Pre-process input based on options
    let input = options.process_crlf(input);

    // First normalize multiline keys
    let normalized = normalize_multiline_keys(&input, options.preserve_crlf());

    let mut entries = Vec::new();
    let mut current_key: Option<(String, usize)> = None;
    let mut value_lines: Vec<String> = Vec::new();
    let mut base_indent: Option<usize> = None;

    // Use split('\n') to preserve \r characters when crlf_preserve_literal is set
    // lines() would strip \r, but we want to keep them in values when preserving
    let lines_iter: Box<dyn Iterator<Item = &str>> = if options.preserve_crlf() {
        Box::new(normalized.split('\n'))
    } else {
        Box::new(normalized.lines())
    };

    let preserve_cr = options.preserve_crlf();

    for line in lines_iter {
        // Count leading whitespace (spaces and tabs)
        let indent = line.len() - trim_start_with_cr_option(line, preserve_cr).len();
        let trimmed = trim_with_cr_option(line, preserve_cr);

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
        // A line is an entry if it has a valid delimiter OR contains '=' (even if invalid in strict mode)
        // This ensures we handle all lines that look like key-value pairs
        let has_equals = trimmed.contains('=');
        if indent <= base && has_equals {
            // Save previous entry if exists
            if let Some((key, key_indent)) = current_key.take() {
                let value = finalize_value(&value_lines.join("\n"), options);
                entries.push(Entry {
                    key,
                    value,
                    indent: key_indent,
                });
                value_lines.clear();
            }

            // Parse new key-value pair using spacing-aware delimiter detection
            if let Some(eq_pos) = find_delimiter(trimmed, options) {
                // Valid delimiter found - split key and value
                // Trim all whitespace from key (spaces and tabs)
                let key = trimmed[..eq_pos].trim().to_string();
                // Trim value based on spacing options
                let value_raw = &trimmed[eq_pos + 1..];
                let value = trim_value(value_raw, options);

                current_key = Some((key, indent));
                if value.is_empty() {
                    // Empty inline value - add empty string so that when continuation
                    // lines are joined with "\n", the value starts with "\n"
                    // e.g., "server =" with indented children should have value "\n  child = ..."
                    value_lines.push(String::new());
                } else {
                    value_lines.push(value);
                }
            } else {
                // No valid delimiter (e.g., "key=value" in strict spacing mode)
                // Treat the entire line as a key with empty value
                current_key = Some((trimmed.to_string(), indent));
                value_lines.push(String::new());
            }
        } else if let Some((_, key_indent)) = current_key {
            // Only treat as continuation if indented MORE than the key line
            if indent > key_indent {
                // This line is indented relative to the key - it's part of the current value
                // Preserve the full line for nested structures (tabs processed later)
                value_lines.push(line.to_string());
            } else {
                // Not indented more than key - save current entry and start new one
                let (key, key_indent_final) = current_key.take().unwrap();
                let value = finalize_value(&value_lines.join("\n"), options);
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
        let value = finalize_value(&value_lines.join("\n"), options);
        entries.push(Entry {
            key,
            value,
            indent: key_indent,
        });
    }

    entries
}

/// Finalize a value by trimming trailing whitespace and processing tabs
fn finalize_value(value: &str, options: &ParserOptions) -> String {
    // Trim trailing whitespace, but preserve \r if crlf_preserve_literal is set
    let trimmed = if options.preserve_crlf() {
        value.trim_end_matches([' ', '\t', '\n'])
    } else {
        value.trim_end()
    };
    options.process_tabs(trimmed).into_owned()
}

/// Build hierarchical structure from flat entries
#[allow(dead_code)]
pub(crate) fn parse_to_map(
    input: &str,
    options: &ParserOptions,
) -> Result<IndexMap<String, Vec<String>>> {
    let entries = parse_entries(input, options);
    let mut result: IndexMap<String, Vec<String>> = IndexMap::new();

    for entry in entries {
        // Preserve indentation as-is per CCL specification
        result.entry(entry.key).or_default().push(entry.value);
    }

    Ok(result)
}

/// Parse CCL input into a flat list of key-value entries preserving insertion order.
///
/// Unlike `parse_to_map`, this returns entries in their original order without
/// grouping by key. This is essential for structure-preserving `print()` which
/// needs to reproduce the original entry interleaving.
pub(crate) fn parse_to_entries(
    input: &str,
    options: &ParserOptions,
) -> Result<Vec<crate::Entry>> {
    let entries = parse_entries(input, options);
    Ok(entries
        .into_iter()
        .map(|e| crate::Entry::new(e.key, e.value))
        .collect())
}

// Unit tests removed - all parser functionality is covered by data-driven tests in:
// - api_core_ccl_parsing.json (basic_key_value_pairs, equals_in_values, multiline_values, etc.)
// - api_core_ccl_hierarchy.json (duplicate_keys_to_lists, nested structures)
// - api_advanced_processing.json (list_with_empty_keys)
// - api_comments.json (comment handling)
// - api_proposed_behavior.json (proposed behavior, currently excluded)

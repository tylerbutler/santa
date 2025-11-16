//! Santa Data - Data models and CCL parser for Santa Package Manager
//!
//! This crate provides:
//! - Core data models (Platform, KnownSources, PackageData, etc.)
//! - CCL schema definitions (PackageDefinition, SourceDefinition, etc.)
//! - CCL parser that handles both simple and complex formats
//!
//! The parser works around limitations in serde_ccl 0.1.1.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

mod parser;
pub mod models;
pub mod schemas;

pub use parser::{parse_ccl, CclValue};
pub use models::*;
pub use schemas::*;

/// Parse CCL string into a HashMap where values can be either arrays or objects
///
/// This function works around serde_ccl's limitation where it returns strings
/// for nested structures instead of properly parsed values.
///
/// # Examples
///
/// ```
/// use santa_data::parse_to_hashmap;
/// use serde_json::Value;
///
/// let ccl = r#"
/// simple_pkg =
///   = brew
///   = scoop
///
/// complex_pkg =
///   _sources =
///     = brew
///   brew = gh
/// "#;
///
/// let result = parse_to_hashmap(ccl).unwrap();
/// assert!(result.contains_key("simple_pkg"));
/// assert!(result.contains_key("complex_pkg"));
/// ```
pub fn parse_to_hashmap(ccl_content: &str) -> Result<HashMap<String, Value>> {
    // First try serde_ccl for the outer structure
    // serde_ccl returns HashMap<String, String> where the values are
    // the raw CCL content as strings
    let raw: HashMap<String, String> =
        serde_ccl::from_str(ccl_content).context("Failed to parse CCL with serde_ccl")?;

    let mut result = HashMap::new();

    for (key, value_str) in raw.into_iter() {
        // Parse the string value as CCL
        let parsed_value = parse_value_string(&value_str)?;
        result.insert(key, parsed_value);
    }

    Ok(result)
}

/// Parse a CCL value string (from serde_ccl's string output) into a proper JSON Value
fn parse_value_string(s: &str) -> Result<Value> {
    let trimmed = s.trim();

    // Check if it's a simple array (starts with '=')
    if trimmed.starts_with('=') {
        return parse_simple_array(trimmed);
    }

    // Check if it contains field assignments (complex object)
    if trimmed.contains('=') {
        return parse_complex_object(trimmed);
    }

    // Fallback: treat as string
    Ok(Value::String(s.to_string()))
}

/// Parse simple array format: "= brew\n  = scoop"
fn parse_simple_array(s: &str) -> Result<Value> {
    let items: Vec<String> = s
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix('=') {
                let value = stripped.trim();
                if !value.is_empty() {
                    Some(value.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(Value::Array(items.into_iter().map(Value::String).collect()))
}

/// Parse complex object format: "_sources =\n  = brew\nbrew = gh"
fn parse_complex_object(s: &str) -> Result<Value> {
    let mut obj = serde_json::Map::new();
    let mut current_key: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut current_indent = 0;

    for line in s.lines() {
        let line_indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Check if this line starts a new field (has '=' and is at base indent or less)
        if let Some(eq_pos) = trimmed.find('=') {
            let is_array_element = trimmed.starts_with('=');

            // If this is an array element and we're collecting, add it
            if is_array_element && current_key.is_some() && line_indent > current_indent {
                current_lines.push(line.to_string());
                continue;
            }

            // This is a new field - process previous field if any
            if let Some(key) = current_key.take() {
                let value_str = current_lines.join("\n");
                let value = parse_value_string(&value_str)?;
                obj.insert(key, value);
                current_lines.clear();
            }

            if is_array_element {
                // Start collecting array elements without a key name
                // This shouldn't happen in well-formed CCL but handle it
                current_lines.push(line.to_string());
                continue;
            }

            // Extract the new field name
            let field_name = trimmed[..eq_pos].trim();
            let field_value = trimmed[eq_pos + 1..].trim();

            current_indent = line_indent;

            if !field_value.is_empty() {
                // Value on same line
                obj.insert(
                    field_name.to_string(),
                    Value::String(field_value.to_string()),
                );
            } else {
                // Value on next lines
                current_key = Some(field_name.to_string());
            }
        } else if current_key.is_some() {
            // Continuation of current field value
            current_lines.push(line.to_string());
        }
    }

    // Process any remaining field
    if let Some(key) = current_key {
        let value_str = current_lines.join("\n");
        let value = parse_value_string(&value_str)?;
        obj.insert(key, value);
    }

    Ok(Value::Object(obj))
}

/// Parse CCL string and deserialize into a specific type
///
/// # Examples
///
/// ```
/// use santa_data::parse_ccl_to;
/// use serde::Deserialize;
/// use std::collections::HashMap;
///
/// #[derive(Deserialize)]
/// struct Package {
///     #[serde(rename = "_sources")]
///     sources: Option<Vec<String>>,
/// }
///
/// let ccl = r#"
/// bat =
///   _sources =
///     = brew
///     = scoop
/// "#;
///
/// let packages: HashMap<String, Package> = parse_ccl_to(ccl).unwrap();
/// assert!(packages.contains_key("bat"));
/// ```
pub fn parse_ccl_to<T: DeserializeOwned>(ccl_content: &str) -> Result<T> {
    let hashmap = parse_to_hashmap(ccl_content)?;
    let value = serde_json::to_value(hashmap)?;
    serde_json::from_value(value).context("Failed to deserialize parsed CCL")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_array() {
        let ccl = r#"
test_pkg =
  = brew
  = scoop
  = pacman
"#;
        let result = parse_to_hashmap(ccl).unwrap();

        assert!(result.contains_key("test_pkg"));
        let value = &result["test_pkg"];
        assert!(value.is_array());

        let arr = value.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_str().unwrap(), "brew");
        assert_eq!(arr[1].as_str().unwrap(), "scoop");
        assert_eq!(arr[2].as_str().unwrap(), "pacman");
    }

    #[test]
    fn test_parse_complex_object() {
        let ccl = r#"
test_pkg =
  _sources =
    = brew
    = scoop
  brew = gh
"#;
        let result = parse_to_hashmap(ccl).unwrap();

        assert!(result.contains_key("test_pkg"));
        let value = &result["test_pkg"];
        println!("Parsed value: {:#?}", value);
        assert!(value.is_object());

        let obj = value.as_object().unwrap();
        println!("Object keys: {:?}", obj.keys().collect::<Vec<_>>());
        assert!(obj.contains_key("_sources"));
        assert!(obj.contains_key("brew"));

        let sources_value = &obj["_sources"];
        println!("_sources value: {:#?}", sources_value);
        let sources = sources_value.as_array().unwrap();
        assert_eq!(sources.len(), 2);

        let brew_override = obj["brew"].as_str().unwrap();
        assert_eq!(brew_override, "gh");
    }

    #[test]
    fn test_parse_multiple_packages() {
        let ccl = r#"
simple =
  = brew
  = scoop

complex =
  _sources =
    = pacman
  _platforms =
    = linux
"#;
        let result = parse_to_hashmap(ccl).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result["simple"].is_array());
        assert!(result["complex"].is_object());
    }
}

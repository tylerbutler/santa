//! Santa Data - Data models, configuration, and CCL parser for Santa Package Manager
//!
//! This crate provides:
//! - Core data models (Platform, KnownSources, PackageData, etc.)
//! - Configuration loading and management (SantaConfig, ConfigLoader)
//! - CCL schema definitions (PackageDefinition, SourceDefinition, etc.)
//! - CCL parser that handles both simple and complex formats

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

pub mod config;
pub mod models;
mod parser;
pub mod schemas;

pub use config::*;
pub use models::*;
pub use parser::{parse_ccl, CclValue};
pub use schemas::*;

/// Parse CCL string into a HashMap where values can be either arrays or objects
///
/// With sickle, this function directly deserializes CCL into proper Value types.
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
    // Parse using sickle and convert Model to JSON Value
    let model = sickle::parse(ccl_content).context("Failed to parse CCL with sickle")?;

    // Convert the model to a HashMap<String, Value>
    model_to_hashmap(&model)
}

/// Convert a sickle Model to a HashMap<String, Value>
fn model_to_hashmap(model: &sickle::Model) -> Result<HashMap<String, Value>> {
    let map = model.as_map().context("Expected root to be a map")?;
    let mut result = HashMap::new();

    for (key, value) in map {
        result.insert(key.clone(), model_to_value(value)?);
    }

    Ok(result)
}

/// Convert a sickle Model to a serde_json Value
fn model_to_value(model: &sickle::Model) -> Result<Value> {
    match model {
        sickle::Model::Singleton(s) => {
            // Check if this string contains list syntax (lines starting with '=')
            if is_list_syntax(s) {
                // Parse it as a list
                parse_list_string(s)
            } else {
                Ok(Value::String(s.clone()))
            }
        }
        sickle::Model::List(items) => {
            let values: Result<Vec<_>> = items.iter().map(model_to_value).collect();
            Ok(Value::Array(values?))
        }
        sickle::Model::Map(map) => {
            // Check if this is actually a list (empty key entries)
            if map.keys().any(|k| k.is_empty()) {
                // This is a list with empty keys - extract the values
                let mut values = Vec::new();
                for (k, v) in map {
                    if k.is_empty() {
                        values.push(model_to_value(v)?);
                    }
                }
                Ok(Value::Array(values))
            } else {
                let mut obj = serde_json::Map::new();
                for (k, v) in map {
                    obj.insert(k.clone(), model_to_value(v)?);
                }
                Ok(Value::Object(obj))
            }
        }
    }
}

/// Check if a string contains CCL list syntax (lines starting with '=')
fn is_list_syntax(s: &str) -> bool {
    s.lines().any(|line| line.trim().starts_with('='))
}

/// Parse a string containing list syntax into a JSON array
fn parse_list_string(s: &str) -> Result<Value> {
    let items: Vec<Value> = s
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(stripped) = trimmed.strip_prefix('=') {
                // Extract the value after '='
                let value = stripped.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(Value::String(value.to_string()))
                }
            } else {
                None
            }
        })
        .collect();

    Ok(Value::Array(items))
}

// Experimental CCL parsing functions used only in tests
#[cfg(test)]
#[allow(dead_code)]
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

#[cfg(test)]
#[allow(dead_code)]
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

#[cfg(test)]
#[allow(dead_code)]
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
        println!("DEBUG test_pkg value: {:#?}", value);
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

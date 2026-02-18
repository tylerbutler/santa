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
    // Parse using sickle's load function (parse + build_hierarchy)
    let model = sickle::load(ccl_content).context("Failed to parse CCL with sickle")?;

    // Convert the model to a HashMap<String, Value>
    model_to_hashmap(&model)
}

/// Convert a sickle Model to a HashMap<String, Value>
fn model_to_hashmap(model: &sickle::CclObject) -> Result<HashMap<String, Value>> {
    let mut result = HashMap::new();

    for (key, value) in model.iter() {
        result.insert(key.clone(), model_to_value(value)?);
    }

    Ok(result)
}

/// Convert a sickle Model to a serde_json Value
fn model_to_value(model: &sickle::CclObject) -> Result<Value> {
    // Check if this is a list with empty keys (CCL: = item1\n = item2)
    // Empty keys have all their values stored in a Vec under the "" key
    if let Ok(empty_key_values) = model.get_all("") {
        if !empty_key_values.is_empty() {
            // Check if all values are simple string values (single key with empty value)
            let all_simple_strings = empty_key_values
                .iter()
                .all(|v| v.len() == 1 && v.values().all(|child| child.is_empty()));

            if all_simple_strings {
                // Extract string values
                let values: Vec<Value> = empty_key_values
                    .iter()
                    .filter_map(|v| v.keys().next().cloned())
                    .map(Value::String)
                    .collect();
                return Ok(Value::Array(values));
            } else {
                // Convert each value recursively
                let values: Vec<Value> = empty_key_values
                    .iter()
                    .map(model_to_value)
                    .collect::<Result<Vec<_>>>()?;
                return Ok(Value::Array(values));
            }
        }
    }

    // Fast path for singleton maps
    if model.len() == 1 {
        let (key, value) = model.iter().next().unwrap();

        // Check if this is a singleton string: {"value": {}}
        if value.is_empty() {
            return Ok(Value::String(key.clone()));
        }
    }

    // Check if this is a list (multiple keys all with empty values)
    if model.len() > 1 && model.values().all(|v| v.is_empty()) {
        // This is a list - keys are the list items
        let values: Vec<Value> = model.keys().map(|k| Value::String(k.clone())).collect();
        return Ok(Value::Array(values));
    }

    // Otherwise, it's a map (object)
    let mut obj = serde_json::Map::new();
    for (k, v) in model.iter() {
        obj.insert(k.clone(), model_to_value(v)?);
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
    // Normalize CRLF to LF for cross-platform compatibility (e.g. Windows checkouts)
    let options =
        sickle::ParserOptions::default().with_crlf(sickle::options::CrlfBehavior::NormalizeToLf);
    sickle::from_str_with_options(ccl_content, &options).context("Failed to deserialize parsed CCL")
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

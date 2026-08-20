//! Santa Data - Data models, configuration, and CCL parser for Santa Package Manager
//!
//! This crate provides:
//! - Core data models (Platform, KnownSources, PackageData, etc.)
//! - Configuration loading and management (SantaConfig, ConfigLoader)
//! - CCL schema definitions (PackageDefinition, SourceDefinition, etc.)
//! - CCL parsing that handles both simple and complex formats

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;

pub mod config;
pub mod models;
pub mod schemas;

pub use config::{
    ConfigLoader, ConfigPackageSource, PackageNameOverride, SantaConfig, SantaConfigBuilder,
    SourceList,
};
pub use models::{
    Arch, CommandName, Distro, KnownSources, PackageData, PackageDataList, PackageName, Platform,
    SourceName, OS,
};
pub use schemas::{
    ComplexPackageDefinition, ConfigDefinition, ConfigSettings, PackageDefinition,
    PlatformOverride, SourceConfig, SourceDefinition, SourceSpecificConfig, SourcesDefinition,
};

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
    let document = sickle::DocumentMut::parse(ccl_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse CCL with sickle: {e}"))?;

    let mut result = HashMap::new();
    for key in document.as_table().unique_keys() {
        let item = document
            .as_table()
            .get_composed(key)
            .expect("key came from the table");
        result.insert(key.to_string(), item_to_value(&item));
    }
    Ok(result)
}

/// Convert a sickle tree node into a `serde_json::Value`
fn item_to_value(item: &sickle::Item) -> Value {
    match item {
        sickle::Item::Value(scalar) => Value::String(scalar.as_str().to_string()),
        sickle::Item::Array(array) => Value::Array(array.iter().map(item_to_value).collect()),
        sickle::Item::Table(table) => {
            let mut object = serde_json::Map::new();
            for key in table.unique_keys() {
                let child = table.get_composed(key).expect("key came from the table");
                object.insert(key.to_string(), item_to_value(&child));
            }
            Value::Object(object)
        }
        sickle::Item::None => Value::Null,
    }
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
    let options = sickle::Options::new().with_crlf(sickle::CrlfBehavior::NormalizeToLf);
    sickle::de::from_str_with(ccl_content, &options).context("Failed to deserialize parsed CCL")
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

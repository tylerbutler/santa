//! CCL Model - the core data structure for navigating parsed CCL documents

use crate::error::{Error, Result};
use indexmap::IndexMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a single parsed entry (key-value pair) from CCL
///
/// This is the output of the `parse()` function, representing a flat list
/// of key-value pairs before hierarchy construction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Entry {
    /// The key (can be empty for list entries)
    pub key: String,
    /// The value (can be multiline or contain nested CCL)
    pub value: String,
}

impl Entry {
    /// Create a new entry
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Represents a parsed CCL document as a recursive map structure
///
/// Following the OCaml implementation: `type t = Fix of t Map.Make(String).t`
///
/// A CCL document is a fixed-point recursive structure where:
/// - Every Model is a map from String to Model
/// - An empty map {} represents a leaf/terminal value
/// - String values are encoded in the recursive structure
/// - Lists are represented as multiple entries with the same key
/// - Uses IndexMap to preserve insertion order
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Model(IndexMap<String, Model>);

impl Model {
    /// Create a new empty model
    pub fn new() -> Self {
        Model(IndexMap::new())
    }

    /// Create a Model from an IndexMap
    /// This is internal-only for crate operations
    pub(crate) fn from_map(map: IndexMap<String, Model>) -> Self {
        Model(map)
    }

    /// Get a value by key, returning an error if the key doesn't exist
    pub fn get(&self, key: &str) -> Result<&Model> {
        self.0
            .get(key)
            .ok_or_else(|| Error::MissingKey(key.to_string()))
    }

    /// Get an iterator over the keys in this model
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    /// Get an iterator over the values in this model
    pub fn values(&self) -> impl Iterator<Item = &Model> {
        self.0.values()
    }

    /// Get an iterator over key-value pairs in this model
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Model)> {
        self.0.iter()
    }

    /// Get the concrete IndexMap iterator for internal use (Serde)
    pub(crate) fn iter_map(&self) -> indexmap::map::Iter<'_, String, Model> {
        self.0.iter()
    }

    /// Get the number of entries in this model
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if this model is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Extract a string value from the model (no key lookup)
    ///
    /// A string value is represented as a map with a single key (the string) and empty value
    /// Example: `{"Alice": {}}` represents the string "Alice"
    pub(crate) fn as_string(&self) -> Result<&str> {
        if self.0.len() == 1 {
            let (key, value) = self.0.iter().next().unwrap();
            if value.0.is_empty() {
                return Ok(key.as_str());
            }
        }
        Err(Error::ValueError(
            "expected single string value (map with one key and empty value)".to_string(),
        ))
    }

    /// Get a string value by key
    ///
    /// Looks up the key and extracts its string representation
    pub fn get_string(&self, key: &str) -> Result<&str> {
        self.get(key)?.as_string()
    }

    /// Extract a boolean value from the model (no key lookup)
    ///
    /// Parses the string representation as a boolean
    pub(crate) fn as_bool(&self) -> Result<bool> {
        let s = self.as_string()?;
        s.parse::<bool>()
            .map_err(|_| Error::ValueError(format!("failed to parse '{}' as bool", s)))
    }

    /// Get a boolean value by key
    pub fn get_bool(&self, key: &str) -> Result<bool> {
        self.get(key)?.as_bool()
    }

    /// Extract an integer value from the model (no key lookup)
    ///
    /// Parses the string representation as an i64
    pub(crate) fn as_int(&self) -> Result<i64> {
        let s = self.as_string()?;
        s.parse::<i64>()
            .map_err(|_| Error::ValueError(format!("failed to parse '{}' as integer", s)))
    }

    /// Get an integer value by key
    pub fn get_int(&self, key: &str) -> Result<i64> {
        self.get(key)?.as_int()
    }

    /// Extract a float value from the model (no key lookup)
    ///
    /// Parses the string representation as an f64
    pub(crate) fn as_float(&self) -> Result<f64> {
        let s = self.as_string()?;
        s.parse::<f64>()
            .map_err(|_| Error::ValueError(format!("failed to parse '{}' as float", s)))
    }

    /// Get a float value by key
    pub fn get_float(&self, key: &str) -> Result<f64> {
        self.get(key)?.as_float()
    }

    /// Extract a list of string values from the model (no key lookup)
    ///
    /// In CCL, lists are represented as maps with multiple keys.
    /// Each key in the map represents one list element.
    /// Example: `{"item1": {}, "item2": {}, "item3": {}}` represents the list ["item1", "item2", "item3"]
    pub(crate) fn as_list(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }

    /// Get a list of string values by key
    ///
    /// Looks up the key and extracts its list representation
    pub fn get_list(&self, key: &str) -> Result<Vec<String>> {
        Ok(self.get(key)?.as_list())
    }

    /// Create a Model from a string value
    ///
    /// Creates the representation `{string: {}}`
    /// This is internal-only for Serde support
    pub(crate) fn from_string(s: impl Into<String>) -> Self {
        let mut map = IndexMap::new();
        map.insert(s.into(), Model::new());
        Model(map)
    }

    /// Extract the inner IndexMap, consuming the Model
    /// This is internal-only for crate operations
    pub(crate) fn into_inner(self) -> IndexMap<String, Model> {
        self.0
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_model() {
        let model = Model::new();
        assert!(model.is_empty());
    }

    #[test]
    fn test_map_navigation() {
        let mut inner = IndexMap::new();
        inner.insert("name".to_string(), Model::new());
        inner.insert("version".to_string(), Model::new());

        let model = Model(inner);
        assert!(model.get("name").is_ok());
        assert!(model.get("version").is_ok());
        assert!(model.get("nonexistent").is_err());
    }
}

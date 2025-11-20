//! CCL Model - the core data structure for navigating parsed CCL documents

use crate::error::{Error, Result};
use indexmap::IndexMap;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Check if a string is a scalar literal (number or boolean)
///
/// CCL distinguishes between string values and scalar literals:
/// - Numbers: integers and floats (e.g., "42", "3.14", "-17")
/// - Booleans: true/false/yes/no
///
/// This helper is used to filter out scalar literals from string lists
/// when `list_coercion_enabled` feature is active.
#[cfg(feature = "list_coercion_enabled")]
fn is_scalar_literal(s: &str) -> bool {
    // Check if it's parseable as an integer
    if s.parse::<i64>().is_ok() {
        return true;
    }

    // Check if it's parseable as a float
    if s.parse::<f64>().is_ok() {
        return true;
    }

    // Check if it's a boolean literal
    matches!(s, "true" | "false" | "yes" | "no")
}

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
    /// In CCL, lists are represented using **bare list syntax** with empty keys:
    /// ```text
    /// servers =
    ///   = web1
    ///   = web2
    /// ```
    ///
    /// Behavior varies by feature flag:
    ///
    /// **With `list_coercion_disabled` (default, reference-compliant)**:
    /// - Returns values ONLY when all keys are empty strings `""`
    /// - Duplicate keys with values are NOT lists: `servers = web1` → `[]`
    /// - Bare lists work: Access via `get("servers")?.get_list("")` → `["web1", "web2"]`
    /// - Matches OCaml reference implementation
    ///
    /// **With `list_coercion_enabled`**:
    /// - Duplicate keys create lists: `servers = web1` → `["web1", "web2"]`
    /// - Still filters scalar literals (numbers/booleans)
    /// - Single values coerced to lists
    pub(crate) fn as_list(&self) -> Vec<String> {
        #[cfg(feature = "list_coercion_disabled")]
        {
            // Filter out comment keys (starting with '/') when checking for bare lists
            let non_comment_keys: Vec<&String> = self.keys().filter(|k| !k.starts_with('/')).collect();

            // Handle bare list syntax: single empty-key child containing the list items
            // Example: servers = { "": { "web1": {}, "web2": {} } }
            // Should return ["web1", "web2"]
            // Also handles: { "": {...}, "/": {...comment...} } - ignores comments
            if non_comment_keys.len() == 1 && non_comment_keys[0].is_empty() {
                if let Some(child) = self.get("").ok() {
                    // Found empty-key child - return its keys as the list
                    // Also filter out comment keys from the child
                    return child.keys().filter(|k| !k.starts_with('/')).cloned().collect();
                }
            }

            // Empty or single non-empty key = not a list
            if non_comment_keys.len() <= 1 {
                return Vec::new();
            }

            // Multiple non-comment keys: ONLY return values if ALL are empty strings
            // This means we're in a bare list structure with multiple empty keys
            let all_keys_empty = non_comment_keys.iter().all(|k| k.is_empty());
            if all_keys_empty {
                // For bare lists, the VALUES (nested keys) are the list items
                // But since keys are empty, we need to look at the nested structure
                // For now, return empty as bare lists need special handling
                Vec::new()
            } else {
                // Non-empty keys = not a list in reference mode
                Vec::new()
            }
        }

        #[cfg(feature = "list_coercion_enabled")]
        {
            // Coercion mode: duplicate keys create lists, but filter scalars
            self.keys()
                .filter(|k| !is_scalar_literal(k))
                .cloned()
                .collect()
        }

        #[cfg(not(any(feature = "list_coercion_disabled", feature = "list_coercion_enabled")))]
        {
            compile_error!("Must enable either 'list_coercion_disabled' or 'list_coercion_enabled'");
        }

        #[cfg(all(feature = "list_coercion_disabled", feature = "list_coercion_enabled"))]
        {
            compile_error!("Cannot enable both 'list_coercion_disabled' and 'list_coercion_enabled' - they are mutually exclusive");
        }
    }

    /// Get a list of string values by key
    ///
    /// Looks up the key and extracts its list representation.
    /// Filters out scalar literals (numbers and booleans).
    ///
    /// For typed access to lists of scalars, use `get_list_typed::<T>()` instead.
    pub fn get_list(&self, key: &str) -> Result<Vec<String>> {
        Ok(self.get(key)?.as_list())
    }

    /// Get a typed list of values by key
    ///
    /// This method provides generic access to lists of any parseable type.
    /// Unlike `get_list()`, this doesn't filter scalar literals - it parses all keys as type T.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sickle::{Model, parse, build_hierarchy};
    /// # use sickle::error::Result;
    /// # fn example() -> Result<()> {
    /// // Numbers list
    /// let input = "numbers = 1\nnumbers = 42\nnumbers = -17";
    /// let entries = parse(input)?;
    /// let model = build_hierarchy(&entries)?;
    /// let numbers: Vec<i64> = model.get_list_typed("numbers")?;
    /// assert_eq!(numbers, vec![1, 42, -17]);
    ///
    /// // Booleans list
    /// let input = "flags = true\nflags = false";
    /// let entries = parse(input)?;
    /// let model = build_hierarchy(&entries)?;
    /// let flags: Vec<bool> = model.get_list_typed("flags")?;
    /// assert_eq!(flags, vec![true, false]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Error::ValueError` if any key cannot be parsed as type T.
    pub fn get_list_typed<T>(&self, key: &str) -> Result<Vec<T>>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        let model = self.get(key)?;

        // For typed lists, we want ALL keys (including scalar literals)
        if model.len() >= 2 {
            model
                .keys()
                .map(|k| {
                    k.parse::<T>().map_err(|e| {
                        Error::ValueError(format!(
                            "Failed to parse '{}' as {}: {}",
                            k,
                            std::any::type_name::<T>(),
                            e
                        ))
                    })
                })
                .collect()
        } else {
            Ok(Vec::new())
        }
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

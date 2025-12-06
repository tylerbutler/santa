//! CCL Model - the core data structure for navigating parsed CCL documents

use crate::error::{Error, Result};
use indexmap::IndexMap;
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Type Aliases - prevent confusion between single-value and multi-value maps
// ============================================================================

/// The internal storage type for CclObject - maps keys to a Vec of values.
/// Each key can have multiple values, preserving duplicate key semantics.
pub(crate) type CclMap = IndexMap<String, Vec<CclObject>>;

/// Iterator over key-value pairs where value is the full Vec.
pub(crate) type CclMapIter<'a> = indexmap::map::Iter<'a, String, Vec<CclObject>>;

/// Options for list access operations
///
/// Controls how `get_list()` interprets the CCL data structure.
#[derive(Debug, Clone, Copy, Default)]
pub struct ListOptions {
    /// When true, duplicate keys are coerced into lists and scalar literals are filtered.
    /// When false (default), only bare list syntax (empty keys) produces lists.
    pub coerce: bool,
}

impl ListOptions {
    /// Create default options (coerce = false)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create options with coercion enabled
    pub fn with_coerce() -> Self {
        Self { coerce: true }
    }
}

/// Check if a string is a scalar literal (number or boolean)
///
/// CCL distinguishes between string values and scalar literals:
/// - Numbers: integers and floats (e.g., "42", "3.14", "-17")
/// - Booleans: true/false/yes/no
///
/// This helper is used to filter out scalar literals from string lists
/// when coercion is enabled.
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
/// Following the OCaml implementation: `type entry_map = value_entry list KeyMap.t`
///
/// A CCL document is a fixed-point recursive structure where:
/// - Every Model is a map from String to Vec<Model>
/// - An empty map {} represents a leaf/terminal value
/// - String values are encoded in the recursive structure
/// - Lists are represented as multiple entries with the same key
/// - Uses IndexMap to preserve insertion order (keys ordered by first appearance)
/// - Uses Vec to preserve order of values for each key (insertion order)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CclObject(CclMap);

impl CclObject {
    /// Create a new empty model
    pub fn new() -> Self {
        CclObject(IndexMap::new())
    }

    /// Create a Model from an IndexMap
    /// This is internal-only for crate operations
    pub(crate) fn from_map(map: CclMap) -> Self {
        CclObject(map)
    }

    /// Get a value by key, returning an error if the key doesn't exist
    ///
    /// If the key has multiple values, returns the first one (matching OCaml behavior).
    /// Use `get_all()` to get all values for a key.
    pub fn get(&self, key: &str) -> Result<&CclObject> {
        self.0
            .get(key)
            .and_then(|vec| vec.first())
            .ok_or_else(|| Error::MissingKey(key.to_string()))
    }

    /// Get all values for a key, returning an error if the key doesn't exist
    pub fn get_all(&self, key: &str) -> Result<&[CclObject]> {
        self.0
            .get(key)
            .map(|vec| vec.as_slice())
            .ok_or_else(|| Error::MissingKey(key.to_string()))
    }

    /// Get an iterator over the keys in this model
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    /// Get an iterator over the first value for each key
    ///
    /// This flattens the Vec structure, returning only the first value per key.
    /// Use `iter_all()` to get all values.
    pub fn values(&self) -> impl Iterator<Item = &CclObject> {
        self.0.values().filter_map(|vec| vec.first())
    }

    /// Get an iterator over key-value pairs (first value only per key)
    ///
    /// This flattens the Vec structure, returning only the first value per key.
    /// Use `iter_all()` to get all key-value pairs including duplicates.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CclObject)> {
        self.0
            .iter()
            .filter_map(|(k, vec)| vec.first().map(|v| (k, v)))
    }

    /// Get an iterator over all key-value pairs including duplicate keys
    pub fn iter_all(&self) -> impl Iterator<Item = (&String, &CclObject)> {
        self.0
            .iter()
            .flat_map(|(k, vec)| vec.iter().map(move |v| (k, v)))
    }

    /// Get the concrete IndexMap iterator for internal use (Serde)
    pub(crate) fn iter_map(&self) -> indexmap::map::Iter<'_, String, Vec<CclObject>> {
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

    // ========================================================================
    // Builder API - Programmatic CCL Construction
    // ========================================================================

    /// Get mutable access to the internal IndexMap for direct manipulation
    ///
    /// This allows programmatic construction of CCL structures when you need
    /// full control over the data model.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::CclObject;
    ///
    /// let mut obj = CclObject::new();
    /// let map = obj.inner_mut();
    /// map.insert("key".to_string(), vec![CclObject::from_string("value")]);
    /// ```
    pub fn inner_mut(&mut self) -> &mut CclMap {
        &mut self.0
    }

    /// Create an empty CclObject (represents an empty value in CCL: `key =`)
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::CclObject;
    ///
    /// let empty = CclObject::empty();
    /// // Represents: key =
    /// ```
    pub fn empty() -> Self {
        CclObject(IndexMap::new())
    }

    /// Create a CclObject representing a list using bare list syntax
    ///
    /// In CCL, a list is represented using the same empty key with multiple values.
    /// Now that we use Vec<CclObject> internally, we can properly support duplicate keys.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::CclObject;
    ///
    /// let list = CclObject::from_list(vec!["brew", "scoop", "pacman"]);
    /// // Represents:
    /// // packages =
    /// //   = brew
    /// //   = scoop
    /// //   = pacman
    /// ```
    pub fn from_list(items: Vec<impl Into<String>>) -> Self {
        let mut map = IndexMap::new();
        let values: Vec<CclObject> = items
            .into_iter()
            .map(|item| CclObject::from_string(item))
            .collect();
        map.insert("".to_string(), values);
        CclObject(map)
    }

    /// Create a CclObject representing a comment
    ///
    /// CCL comments use the `/=` prefix. This is a convenience method for
    /// creating comment entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::CclObject;
    ///
    /// let mut obj = CclObject::new();
    /// obj.inner_mut().insert("/= Header".to_string(), vec![CclObject::comment("Generated file")]);
    /// // Represents: /= Generated file
    /// ```
    pub fn comment(text: impl Into<String>) -> Self {
        CclObject::from_string(text)
    }

    // ========================================================================
    // Composition API - CCL Monoid Operations
    // ========================================================================

    /// Compose two CCL objects together (monoid binary operation)
    ///
    /// This implements the fundamental CCL composition operation that makes CCL
    /// a monoid. When composing two objects:
    /// - Keys unique to either object are preserved
    /// - Keys present in both are recursively composed
    /// - The empty object is the identity element
    ///
    /// # Algebraic Properties
    ///
    /// - **Associativity**: `a.compose(&b).compose(&c) == a.compose(&b.compose(&c))`
    /// - **Left Identity**: `CclObject::new().compose(&x) == x`
    /// - **Right Identity**: `x.compose(&CclObject::new()) == x`
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::{load, CclObject};
    ///
    /// let a = load("config =\n  host = localhost").unwrap();
    /// let b = load("config =\n  port = 8080").unwrap();
    /// let composed = a.compose(&b);
    /// // Result: config = { host = localhost, port = 8080 }
    /// ```
    pub fn compose(&self, other: &CclObject) -> CclObject {
        let mut result: CclMap = CclMap::new();

        // First, add all keys from self
        for (key, self_values) in &self.0 {
            if let Some(other_values) = other.0.get(key) {
                // Key exists in both - compose the values
                let composed_values = Self::compose_value_lists(self_values, other_values);
                result.insert(key.clone(), composed_values);
            } else {
                // Key only in self
                result.insert(key.clone(), self_values.clone());
            }
        }

        // Add keys only in other
        for (key, other_values) in &other.0 {
            if !self.0.contains_key(key) {
                result.insert(key.clone(), other_values.clone());
            }
        }

        CclObject(result)
    }

    /// Compose two value lists (Vec<CclObject>) into one
    ///
    /// When composing values for the same key, we merge them into a single
    /// composed value by recursively composing each pair.
    fn compose_value_lists(a: &[CclObject], b: &[CclObject]) -> Vec<CclObject> {
        // For composition, we merge all values from both lists into a single composed value
        // This matches OCaml's behavior: fold_left merge empty [v1, v2, v3, ...]

        // Start with empty, compose all from a, then all from b
        let mut composed = CclObject::new();

        for obj in a {
            composed = composed.compose(obj);
        }
        for obj in b {
            composed = composed.compose(obj);
        }

        vec![composed]
    }

    /// Check if composing three objects is associative
    ///
    /// Tests: `(a ∘ b) ∘ c == a ∘ (b ∘ c)`
    ///
    /// This is used for testing the algebraic properties of CCL.
    pub fn compose_associative(a: &CclObject, b: &CclObject, c: &CclObject) -> bool {
        let left = a.compose(b).compose(c);
        let right = a.compose(&b.compose(c));
        left == right
    }

    /// Check left identity property
    ///
    /// Tests: `empty ∘ x == x`
    pub fn identity_left(x: &CclObject) -> bool {
        let empty = CclObject::new();
        empty.compose(x) == *x
    }

    /// Check right identity property
    ///
    /// Tests: `x ∘ empty == x`
    pub fn identity_right(x: &CclObject) -> bool {
        let empty = CclObject::new();
        x.compose(&empty) == *x
    }

    /// Extract a string value from the model (no key lookup)
    ///
    /// A string value is represented as a map with a single key (the string) and empty value.
    /// Example: `{"Alice": [{}]}` represents the string "Alice"
    pub(crate) fn as_string(&self) -> Result<&str> {
        if self.0.len() == 1 {
            let (key, vec) = self.0.iter().next().unwrap();
            if vec.len() == 1 && vec[0].0.is_empty() {
                return Ok(key.as_str());
            }
        }
        Err(Error::ValueError(
            "expected single string value (map with one key and single empty value)".to_string(),
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
    /// Behavior varies by `options.coerce`:
    ///
    /// **With `coerce = false` (default, reference-compliant)**:
    /// - Returns values ONLY when all keys are empty strings `""`
    /// - Duplicate keys with values are NOT lists: `servers = web1` → `[]`
    /// - Bare lists work: Access via `get("servers")?.as_list_with_options(...)` → `["web1", "web2"]`
    /// - Matches OCaml reference implementation
    ///
    /// **With `coerce = true`**:
    /// - Duplicate keys create lists: `servers = web1` → `["web1", "web2"]`
    /// - Still filters scalar literals (numbers/booleans)
    /// - Single values coerced to lists
    pub(crate) fn as_list_with_options(&self, options: ListOptions) -> Vec<String> {
        if options.coerce {
            // Coercion mode: duplicate keys create lists, but filter scalars
            self.keys()
                .filter(|k| !is_scalar_literal(k))
                .cloned()
                .collect()
        } else {
            // Reference-compliant mode: only bare list syntax works
            // Filter out comment keys (starting with '/') when checking for bare lists
            let non_comment_keys: Vec<&String> =
                self.keys().filter(|k| !k.starts_with('/')).collect();

            // Handle bare list syntax: single empty-key child containing the list items
            // With Vec structure: { "": [CclObject({item1}), CclObject({item2}), ...] }
            // We need to get ALL values from the Vec at key ""
            if non_comment_keys.len() == 1 && non_comment_keys[0].is_empty() {
                if let Ok(children) = self.get_all("") {
                    // Found empty-key entries - each child contains one list item as its key
                    // Filter out comment keys from each child
                    return children
                        .iter()
                        .flat_map(|child| child.keys().filter(|k| !k.starts_with('/')).cloned())
                        .collect();
                }
            }

            // Empty or single non-empty key = not a list
            if non_comment_keys.len() <= 1 {
                return Vec::new();
            }

            // Multiple non-comment keys = not a bare list in reference mode
            Vec::new()
        }
    }

    /// Get a list of string values by key (reference-compliant behavior)
    ///
    /// Only bare list syntax produces lists. Duplicate keys with values are NOT
    /// treated as lists.
    ///
    /// For typed access to lists of scalars, use `get_list_typed::<T>()` instead.
    /// For coercion behavior, use `get_list_coerced()`.
    pub fn get_list(&self, key: &str) -> Result<Vec<String>> {
        Ok(self.get(key)?.as_list_with_options(ListOptions::new()))
    }

    /// Get a list of string values by key (with coercion)
    ///
    /// Duplicate keys are coerced into lists, and scalar literals are filtered.
    ///
    /// For typed access to lists of scalars, use `get_list_typed::<T>()` instead.
    /// For reference-compliant behavior, use `get_list()`.
    pub fn get_list_coerced(&self, key: &str) -> Result<Vec<String>> {
        Ok(self
            .get(key)?
            .as_list_with_options(ListOptions::with_coerce()))
    }

    /// Get a typed list of values by key
    ///
    /// This method provides generic access to lists of any parseable type.
    /// Unlike `get_list()`, this doesn't filter scalar literals - it parses all keys as type T.
    ///
    /// # Examples
    ///
    /// ```
    /// # use sickle::{CclObject, parse, build_hierarchy};
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

    /// Create a CclObject representing a string value
    ///
    /// In CCL, a string is represented as a map with a single key (the string)
    /// and an empty value: `{"string_value": [{}]}`
    ///
    /// # Example
    ///
    /// ```rust
    /// use sickle::CclObject;
    ///
    /// let val = CclObject::from_string("hello");
    /// // Represents: key = hello
    /// ```
    pub fn from_string(s: impl Into<String>) -> Self {
        let mut map = IndexMap::new();
        map.insert(s.into(), vec![CclObject::new()]);
        CclObject(map)
    }

    /// Extract the inner IndexMap, consuming the Model
    /// This is internal-only for crate operations
    #[allow(dead_code)]
    pub(crate) fn into_inner(self) -> CclMap {
        self.0
    }

    /// Insert a string value at the given key
    /// Creates the CCL representation: `{key: {value: {}}}`
    #[cfg(feature = "serde-serialize")]
    pub(crate) fn insert_string(&mut self, key: &str, value: String) {
        let mut inner = IndexMap::new();
        inner.insert(value, vec![CclObject::new()]);
        self.0.insert(key.to_string(), vec![CclObject(inner)]);
    }

    /// Insert a list of string values at the given key
    /// Creates the CCL representation: `{key: {item1: {}, item2: {}, ...}}`
    #[cfg(feature = "serde-serialize")]
    pub(crate) fn insert_list(&mut self, key: &str, values: Vec<String>) {
        let mut inner = IndexMap::new();
        for value in values {
            inner.insert(value, vec![CclObject::new()]);
        }
        self.0.insert(key.to_string(), vec![CclObject(inner)]);
    }

    /// Insert a nested object at the given key
    #[cfg(feature = "serde-serialize")]
    pub(crate) fn insert_object(&mut self, key: &str, obj: CclObject) {
        self.0.insert(key.to_string(), vec![obj]);
    }
}

impl Default for CclObject {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_model() {
        let model = CclObject::new();
        assert!(model.is_empty());
    }

    #[test]
    fn test_map_navigation() {
        let mut inner = IndexMap::new();
        inner.insert("name".to_string(), vec![CclObject::new()]);
        inner.insert("version".to_string(), vec![CclObject::new()]);

        let model = CclObject(inner);
        assert!(model.get("name").is_ok());
        assert!(model.get("version").is_ok());
        assert!(model.get("nonexistent").is_err());
    }

    #[test]
    fn test_compose_disjoint_keys() {
        // Composing objects with different keys should combine them
        let a = CclObject::from_string("hello");
        let b = CclObject::from_string("world");

        let mut obj_a = CclObject::new();
        obj_a.inner_mut().insert("a".to_string(), vec![a]);

        let mut obj_b = CclObject::new();
        obj_b.inner_mut().insert("b".to_string(), vec![b]);

        let composed = obj_a.compose(&obj_b);
        assert!(composed.get("a").is_ok());
        assert!(composed.get("b").is_ok());
    }

    #[test]
    fn test_compose_overlapping_keys() {
        // Composing objects with same key should merge values
        let mut obj_a = CclObject::new();
        obj_a.inner_mut().insert(
            "config".to_string(),
            vec![{
                let mut inner = CclObject::new();
                inner.inner_mut().insert(
                    "host".to_string(),
                    vec![CclObject::from_string("localhost")],
                );
                inner
            }],
        );

        let mut obj_b = CclObject::new();
        obj_b.inner_mut().insert(
            "config".to_string(),
            vec![{
                let mut inner = CclObject::new();
                inner
                    .inner_mut()
                    .insert("port".to_string(), vec![CclObject::from_string("8080")]);
                inner
            }],
        );

        let composed = obj_a.compose(&obj_b);
        let config = composed.get("config").unwrap();
        assert!(config.get("host").is_ok());
        assert!(config.get("port").is_ok());
    }

    #[test]
    fn test_compose_left_identity() {
        let mut obj = CclObject::new();
        obj.inner_mut()
            .insert("key".to_string(), vec![CclObject::from_string("value")]);

        assert!(CclObject::identity_left(&obj));
    }

    #[test]
    fn test_compose_right_identity() {
        let mut obj = CclObject::new();
        obj.inner_mut()
            .insert("key".to_string(), vec![CclObject::from_string("value")]);

        assert!(CclObject::identity_right(&obj));
    }

    #[test]
    fn test_compose_associativity() {
        let mut a = CclObject::new();
        a.inner_mut()
            .insert("a".to_string(), vec![CclObject::from_string("1")]);

        let mut b = CclObject::new();
        b.inner_mut()
            .insert("b".to_string(), vec![CclObject::from_string("2")]);

        let mut c = CclObject::new();
        c.inner_mut()
            .insert("c".to_string(), vec![CclObject::from_string("3")]);

        assert!(CclObject::compose_associative(&a, &b, &c));
    }

    #[test]
    fn test_compose_nested_associativity() {
        // Test associativity with overlapping nested keys
        let mut a = CclObject::new();
        a.inner_mut().insert(
            "config".to_string(),
            vec![{
                let mut inner = CclObject::new();
                inner.inner_mut().insert(
                    "host".to_string(),
                    vec![CclObject::from_string("localhost")],
                );
                inner
            }],
        );

        let mut b = CclObject::new();
        b.inner_mut().insert(
            "config".to_string(),
            vec![{
                let mut inner = CclObject::new();
                inner
                    .inner_mut()
                    .insert("port".to_string(), vec![CclObject::from_string("8080")]);
                inner
            }],
        );

        let mut c = CclObject::new();
        c.inner_mut().insert(
            "db".to_string(),
            vec![{
                let mut inner = CclObject::new();
                inner
                    .inner_mut()
                    .insert("name".to_string(), vec![CclObject::from_string("test")]);
                inner
            }],
        );

        assert!(CclObject::compose_associative(&a, &b, &c));
    }
}

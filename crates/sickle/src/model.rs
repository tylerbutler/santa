//! CCL Model - the core data structure for navigating parsed CCL documents

use crate::error::{Error, Result};
use std::collections::BTreeMap;
use std::str::FromStr;

/// Represents a single parsed entry (key-value pair) from CCL
///
/// This is the output of the `parse()` function, representing a flat list
/// of key-value pairs before hierarchy construction.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Represents a parsed CCL document as a hierarchical structure
///
/// A CCL document is fundamentally a map from strings to other CCL documents.
/// Values can be:
/// - Singleton strings
/// - Lists of values
/// - Nested maps (objects)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Model {
    /// A single string value
    Singleton(String),

    /// A list of models (from empty keys or valueless keys)
    List(Vec<Model>),

    /// A map of keys to models
    Map(BTreeMap<String, Model>),
}

impl Model {
    /// Create a new empty map model
    pub fn new() -> Self {
        Model::Map(BTreeMap::new())
    }

    /// Create a singleton value
    pub fn singleton(value: impl Into<String>) -> Self {
        Model::Singleton(value.into())
    }

    /// Create a list
    pub fn list(items: Vec<Model>) -> Self {
        Model::List(items)
    }

    /// Get a value by key, returning an error if the key doesn't exist
    pub fn get(&self, key: &str) -> Result<&Model> {
        match self {
            Model::Map(map) => map
                .get(key)
                .ok_or_else(|| Error::MissingKey(key.to_string())),
            _ => Err(Error::NotAMap),
        }
    }

    /// Get a nested value using a path like "database.host"
    pub fn at(&self, path: &str) -> Result<&Model> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for part in parts {
            current = current.get(part)?;
        }

        Ok(current)
    }

    /// Get the value as a string if it's a singleton
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Model::Singleton(s) => Ok(s.as_str()),
            _ => Err(Error::NotASingleton),
        }
    }

    /// Get the value as a list if it's a list
    pub fn as_list(&self) -> Result<&[Model]> {
        match self {
            Model::List(list) => Ok(list.as_slice()),
            _ => Err(Error::NotAList),
        }
    }

    /// Get the value as a map if it's a map
    pub fn as_map(&self) -> Result<&BTreeMap<String, Model>> {
        match self {
            Model::Map(map) => Ok(map),
            _ => Err(Error::NotAMap),
        }
    }

    /// Parse a singleton value as a specific type using FromStr
    pub fn parse_value<T>(&self) -> Result<T>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        let s = self.as_str()?;
        s.parse::<T>()
            .map_err(|e| Error::ValueError(format!("failed to parse '{}': {}", s, e)))
    }

    /// Check if this is a singleton value
    pub fn is_singleton(&self) -> bool {
        matches!(self, Model::Singleton(_))
    }

    /// Check if this is a list
    pub fn is_list(&self) -> bool {
        matches!(self, Model::List(_))
    }

    /// Check if this is a map
    pub fn is_map(&self) -> bool {
        matches!(self, Model::Map(_))
    }

    /// Merge another model into this one
    ///
    /// Merging rules:
    /// - Map + Map = merged map (recursive)
    /// - List + List = concatenated list
    /// - Singleton + Singleton = List of both
    /// - Map + other = Map wins
    pub fn merge(self, other: Model) -> Model {
        match (self, other) {
            (Model::Map(mut left), Model::Map(right)) => {
                for (key, value) in right {
                    left.entry(key)
                        .and_modify(|existing| {
                            *existing = existing.clone().merge(value.clone());
                        })
                        .or_insert(value);
                }
                Model::Map(left)
            }
            (Model::List(mut left), Model::List(right)) => {
                left.extend(right);
                Model::List(left)
            }
            (Model::Singleton(left), Model::Singleton(right)) => {
                Model::List(vec![Model::Singleton(left), Model::Singleton(right)])
            }
            (map @ Model::Map(_), _) => map,
            (_, map @ Model::Map(_)) => map,
            (list @ Model::List(_), _) => list,
            (_, list @ Model::List(_)) => list,
        }
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
    fn test_singleton_creation() {
        let model = Model::singleton("hello");
        assert!(model.is_singleton());
        assert_eq!(model.as_str().unwrap(), "hello");
    }

    #[test]
    fn test_list_creation() {
        let model = Model::list(vec![Model::singleton("item1"), Model::singleton("item2")]);
        assert!(model.is_list());
        assert_eq!(model.as_list().unwrap().len(), 2);
    }

    #[test]
    fn test_map_navigation() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), Model::singleton("Santa"));
        map.insert("version".to_string(), Model::singleton("0.1.0"));

        let model = Model::Map(map);
        assert_eq!(model.get("name").unwrap().as_str().unwrap(), "Santa");
        assert_eq!(model.get("version").unwrap().as_str().unwrap(), "0.1.0");
    }

    #[test]
    fn test_nested_navigation() {
        let mut inner = BTreeMap::new();
        inner.insert("host".to_string(), Model::singleton("localhost"));

        let mut outer = BTreeMap::new();
        outer.insert("database".to_string(), Model::Map(inner));

        let model = Model::Map(outer);
        assert_eq!(
            model.at("database.host").unwrap().as_str().unwrap(),
            "localhost"
        );
    }

    #[test]
    fn test_parse_value() {
        let model = Model::singleton("42");
        let num: i32 = model.parse_value().unwrap();
        assert_eq!(num, 42);
    }

    #[test]
    fn test_merge_maps() {
        let mut map1 = BTreeMap::new();
        map1.insert("a".to_string(), Model::singleton("1"));

        let mut map2 = BTreeMap::new();
        map2.insert("b".to_string(), Model::singleton("2"));

        let result = Model::Map(map1).merge(Model::Map(map2));
        let result_map = result.as_map().unwrap();

        assert_eq!(result_map.len(), 2);
        assert!(result_map.contains_key("a"));
        assert!(result_map.contains_key("b"));
    }
}

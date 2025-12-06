//! Serde deserialization support for CCL
//!
//! This module provides Serde integration, allowing CCL to be deserialized
//! into Rust structs using the standard `#[derive(Deserialize)]` pattern.

use crate::{CclObject, Error, Result};
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use std::fmt;

/// Deserialize a CCL string into a type T
///
/// # Examples
///
/// ```rust,ignore
/// use serde::Deserialize;
/// use sickle::from_str;
///
/// #[derive(Deserialize)]
/// struct Config {
///     name: String,
///     version: String,
/// }
///
/// let ccl = r#"
/// name = MyApp
/// version = 1.0.0
/// "#;
///
/// let config: Config = from_str(ccl).unwrap();
/// ```
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let model = crate::load(s)?;
    let mut deserializer = Deserializer::from_model(model);
    T::deserialize(&mut deserializer).map_err(|e| Error::ValueError(e.to_string()))
}

/// Deserialize a Model into a type T
pub fn from_model<'de, T>(model: CclObject) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::from_model(model);
    T::deserialize(&mut deserializer).map_err(|e| Error::ValueError(e.to_string()))
}

/// A structure that deserializes CCL into Rust values
pub struct Deserializer {
    model: CclObject,
}

impl Deserializer {
    /// Create a new deserializer from a Model
    pub fn from_model(model: CclObject) -> Self {
        Deserializer { model }
    }
}

/// Helper to extract a string value from the recursive map structure
/// Converts our Result type to DeError for serde compatibility
fn extract_string_value(model: &CclObject) -> std::result::Result<&str, DeError> {
    model
        .as_string()
        .map_err(|e| DeError::custom(e.to_string()))
}

impl<'de> de::Deserializer<'de> for &mut Deserializer {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Check if it's a simple string value (single key with empty map)
        if let Ok(s) = extract_string_value(&self.model) {
            return visitor.visit_str(s);
        }

        // Check if it's a list (multiple keys at same level)
        if self.model.len() > 1 && self.model.values().all(|v| v.is_empty()) {
            return self.deserialize_seq(visitor);
        }

        // Check if it's a list with empty keys (CCL list syntax: = item1, = item2)
        if self.model.len() == 1 {
            if let Ok(value) = self.model.get("") {
                // Even a single-element list should be treated as a sequence
                if !value.is_empty() && value.values().all(|v| v.is_empty()) {
                    return self.deserialize_seq(visitor);
                }
            }
        }

        // Otherwise it's a map
        self.deserialize_map(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let b = self
            .model
            .as_bool()
            .map_err(|e| DeError::custom(e.to_string()))?;
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<i8>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as i8", s)))?;
        visitor.visit_i8(n)
    }

    fn deserialize_i16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<i16>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as i16", s)))?;
        visitor.visit_i16(n)
    }

    fn deserialize_i32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<i32>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as i32", s)))?;
        visitor.visit_i32(n)
    }

    fn deserialize_i64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<i64>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as i64", s)))?;
        visitor.visit_i64(n)
    }

    fn deserialize_u8<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<u8>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as u8", s)))?;
        visitor.visit_u8(n)
    }

    fn deserialize_u16<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<u16>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as u16", s)))?;
        visitor.visit_u16(n)
    }

    fn deserialize_u32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<u32>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as u32", s)))?;
        visitor.visit_u32(n)
    }

    fn deserialize_u64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<u64>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as u64", s)))?;
        visitor.visit_u64(n)
    }

    fn deserialize_f32<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<f32>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as f32", s)))?;
        visitor.visit_f32(n)
    }

    fn deserialize_f64<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let n = s
            .parse::<f64>()
            .map_err(|_| DeError::custom(format!("failed to parse '{}' as f64", s)))?;
        visitor.visit_f64(n)
    }

    fn deserialize_char<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        let mut chars = s.chars();
        let c = chars
            .next()
            .ok_or_else(|| DeError::custom("empty string"))?;
        if chars.next().is_some() {
            return Err(DeError::custom("string too long for char"));
        }
        visitor.visit_char(c)
    }

    fn deserialize_str<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        visitor.visit_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        visitor.visit_bytes(s.as_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Check if it's a list (multiple keys with empty values)
        if self.model.len() > 1 && self.model.values().all(|v| v.is_empty()) {
            // Collect keys as the list items
            let items: Vec<String> = self.model.keys().cloned().collect();
            let seq = StringSeqDeserializer {
                iter: items.into_iter(),
            };
            return visitor.visit_seq(seq);
        }

        // Check if it's a singleton containing list syntax
        if let Ok(s) = self.model.as_string() {
            if is_list_syntax(s) {
                // Parse as a simple list of strings
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
                let seq = StringSeqDeserializer {
                    iter: items.into_iter(),
                };
                return visitor.visit_seq(seq);
            }
        }

        // Check if it's a map with empty keys (list representation)
        if self.model.keys().any(|k| k.is_empty()) {
            // Get all values for the empty key
            if let Ok(values) = self.model.get_all("") {
                // Check if all values are simple string values (each has exactly one key with empty children)
                let all_simple_strings = values
                    .iter()
                    .all(|v| v.len() == 1 && v.values().all(|child| child.is_empty()));

                if all_simple_strings {
                    // Extract the string values from each entry
                    let items: Vec<String> = values
                        .iter()
                        .filter_map(|v| v.keys().next().cloned())
                        .collect();
                    let seq = StringSeqDeserializer {
                        iter: items.into_iter(),
                    };
                    return visitor.visit_seq(seq);
                } else {
                    // Not simple strings, return the CclObjects
                    let list: Vec<crate::CclObject> = values.to_vec();
                    let seq = ModelSeqDeserializer {
                        iter: list.into_iter(),
                    };
                    return visitor.visit_seq(seq);
                }
            }
        }

        Err(DeError::custom("expected a list"))
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let map_de = MapDeserializer {
            iter: self.model.iter_map(),
            value: None,
            full_vec: None,
        };
        visitor.visit_map(map_de)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = extract_string_value(&self.model)?;
        visitor.visit_enum(s.into_deserializer())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

/// Check if a string contains CCL list syntax (lines starting with '=')
fn is_list_syntax(s: &str) -> bool {
    s.lines().any(|line| line.trim().starts_with('='))
}

struct StringSeqDeserializer {
    iter: std::vec::IntoIter<String>,
}

struct ModelSeqDeserializer {
    iter: std::vec::IntoIter<CclObject>,
}

impl<'de> SeqAccess<'de> for StringSeqDeserializer {
    type Error = DeError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(s) => {
                let model = crate::CclObject::from_string(s.clone());
                let mut de = Deserializer { model };
                seed.deserialize(&mut de).map(Some)
            }
            None => Ok(None),
        }
    }
}

impl<'de> SeqAccess<'de> for ModelSeqDeserializer {
    type Error = DeError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(model) => {
                let mut de = Deserializer { model };
                seed.deserialize(&mut de).map(Some)
            }
            None => Ok(None),
        }
    }
}

struct MapDeserializer<'a> {
    iter: crate::model::CclMapIter<'a>,
    value: Option<&'a CclObject>,
    full_vec: Option<&'a Vec<CclObject>>,
}

impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a> {
    type Error = DeError;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, vec)) => {
                // Take the first value from the Vec (serde expects single values per key)
                self.value = vec.first();
                // Store the full Vec for potential list deserialization
                self.full_vec = Some(vec);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        // If there are multiple values in the Vec, compose them into one for deserialization
        // This handles the case of duplicate keys becoming a list
        if let Some(vec) = self.full_vec.take() {
            if vec.len() > 1 {
                // Multiple values for this key - compose them into a single object
                // that can be deserialized as a sequence
                let composed = vec
                    .iter()
                    .fold(CclObject::new(), |acc, obj| acc.compose(obj));
                let mut de = Deserializer { model: composed };
                return seed.deserialize(&mut de);
            }
        }

        match self.value.take() {
            Some(value) => {
                let mut de = Deserializer {
                    model: value.clone(),
                };
                seed.deserialize(&mut de)
            }
            None => Err(DeError::custom("value is missing")),
        }
    }
}

/// Custom error type for deserialization
#[derive(Debug, Clone)]
pub struct DeError {
    msg: String,
}

impl DeError {
    fn custom(msg: impl fmt::Display) -> Self {
        DeError {
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for DeError {}

impl de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DeError::custom(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_deserialize_simple_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            version: String,
        }

        let ccl = r#"
name = Santa
version = 0.1.0
"#;

        let config: Config = from_str(ccl).unwrap();
        assert_eq!(config.name, "Santa");
        assert_eq!(config.version, "0.1.0");
    }

    #[test]
    fn test_deserialize_with_numbers() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Config {
            port: u16,
            timeout: u32,
            enabled: bool,
        }

        let ccl = r#"
port = 8080
timeout = 3000
enabled = true
"#;

        let config: Config = from_str(ccl).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.timeout, 3000);
        assert!(config.enabled);
    }

    #[test]
    fn test_deserialize_hashmap() {
        use std::collections::HashMap;

        #[derive(Deserialize, Debug, PartialEq)]
        struct SourceDef {
            emoji: String,
            install: String,
            check: String,
        }

        let ccl = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew leaves

npm =
  emoji = 📦
  install = npm install -g {package}
  check = npm list -g
"#;

        let sources: HashMap<String, SourceDef> = from_str(ccl).unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains_key("brew"));
        assert!(sources.contains_key("npm"));

        let brew = &sources["brew"];
        assert_eq!(brew.emoji, "🍺");
        assert_eq!(brew.install, "brew install {package}");
        assert_eq!(brew.check, "brew leaves");

        let npm = &sources["npm"];
        assert_eq!(npm.emoji, "📦");
        assert_eq!(npm.install, "npm install -g {package}");
        assert_eq!(npm.check, "npm list -g");
    }

    #[test]
    fn test_deserialize_hashmap_with_optionals() {
        use std::collections::HashMap;

        #[allow(dead_code)]
        #[derive(Deserialize, Debug)]
        struct PlatformOverride {
            install: Option<String>,
            check: Option<String>,
        }

        #[allow(dead_code)]
        #[derive(Deserialize, Debug)]
        struct SourceDef {
            emoji: String,
            install: String,
            check: String,
            prefix: Option<String>,
            #[serde(rename = "_overrides")]
            overrides: Option<HashMap<String, PlatformOverride>>,
        }

        let ccl = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew leaves

npm =
  emoji = 📦
  install = npm install -g {package}
  check = npm list -g
  _overrides =
    windows =
      check = npm root -g | gci -Name

nix =
  emoji = 📈
  install = nix-env -iA {package}
  check = nix-env -q
  prefix = nixpkgs.
"#;

        let sources: HashMap<String, SourceDef> = from_str(ccl).unwrap();
        assert_eq!(sources.len(), 3);

        // Test npm with overrides
        let npm = &sources["npm"];
        assert_eq!(npm.emoji, "📦");
        assert!(npm.overrides.is_some());

        // Test nix with prefix
        let nix = &sources["nix"];
        assert_eq!(nix.prefix, Some("nixpkgs.".to_string()));
    }

    #[test]
    fn test_exact_santa_cli_case() {
        use std::collections::HashMap;

        #[allow(dead_code)]
        #[derive(Deserialize, Debug)]
        struct PlatformOverride {
            install: Option<String>,
            check: Option<String>,
        }

        #[allow(dead_code)]
        #[derive(Deserialize, Debug)]
        struct SourceDef {
            emoji: String,
            install: String,
            check: String,
            prefix: Option<String>,
            #[serde(rename = "_overrides")]
            overrides: Option<HashMap<String, PlatformOverride>>,
        }

        // Exact CCL from failing test
        let ccl = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew leaves --installed-on-request

npm =
  emoji = 📦
  install = npm install -g {package}
  check = npm list -g --depth=0

flathub =
  emoji = 📦
  install = flatpak install flathub {package}
  check = flatpak list --app
"#;

        let result: Result<HashMap<String, SourceDef>> = from_str(ccl);
        match &result {
            Ok(sources) => {
                assert_eq!(sources.len(), 3);
                assert!(sources.contains_key("brew"));
                assert!(sources.contains_key("npm"));
                assert!(sources.contains_key("flathub"));
            }
            Err(e) => {
                panic!("Failed to deserialize: {:?}", e);
            }
        }
    }

    #[test]
    fn test_value_with_equals_sign() {
        #[derive(Deserialize, Debug)]
        struct Config {
            command: String,
        }

        // Test with = in the value
        let ccl = r#"
command = npm list --depth=0
"#;

        let result: Result<Config> = from_str(ccl);
        match &result {
            Ok(config) => {
                assert_eq!(config.command, "npm list --depth=0");
            }
            Err(e) => {
                panic!("Failed to deserialize value with =: {:?}", e);
            }
        }
    }
}

/// Comprehensive serde_test validation for the CCL deserializer.
///
/// These tests verify that the deserializer correctly handles all Serde data types
/// by testing the actual deserialization from CCL strings rather than token sequences,
/// since CCL has its own format that doesn't map directly to Serde tokens.
#[cfg(test)]
mod serde_validation_tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    // ===========================================
    // Primitive Types
    // ===========================================

    #[test]
    fn test_bool_true() {
        let ccl = "value = true";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: bool,
        }
        let s: S = from_str(ccl).unwrap();
        assert!(s.value);
    }

    #[test]
    fn test_bool_false() {
        let ccl = "value = false";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: bool,
        }
        let s: S = from_str(ccl).unwrap();
        assert!(!s.value);
    }

    #[test]
    fn test_i8() {
        let ccl = "value = -128";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: i8,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, -128);
    }

    #[test]
    fn test_i16() {
        let ccl = "value = -32768";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: i16,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, -32768);
    }

    #[test]
    fn test_i32() {
        let ccl = "value = -2147483648";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: i32,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, -2147483648);
    }

    #[test]
    fn test_i64() {
        let ccl = "value = -9223372036854775808";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: i64,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, -9223372036854775808);
    }

    #[test]
    fn test_u8() {
        let ccl = "value = 255";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: u8,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, 255);
    }

    #[test]
    fn test_u16() {
        let ccl = "value = 65535";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: u16,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, 65535);
    }

    #[test]
    fn test_u32() {
        let ccl = "value = 4294967295";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: u32,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, 4294967295);
    }

    #[test]
    fn test_u64() {
        let ccl = "value = 18446744073709551615";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: u64,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, 18446744073709551615);
    }

    #[test]
    fn test_f32() {
        let ccl = "value = 3.14";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: f32,
        }
        let s: S = from_str(ccl).unwrap();
        assert!((s.value - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_f64() {
        let ccl = "value = 3.141592653589793";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: f64,
        }
        let s: S = from_str(ccl).unwrap();
        assert!((s.value - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_char() {
        let ccl = "value = X";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: char,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, 'X');
    }

    #[test]
    fn test_string() {
        let ccl = "value = hello world";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: String,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, "hello world");
    }

    // ===========================================
    // Option Types
    // ===========================================

    #[test]
    fn test_option_some() {
        let ccl = "value = present";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            value: Option<String>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, Some("present".to_string()));
    }

    #[test]
    fn test_option_none_missing_field() {
        let ccl = "other = something";
        #[derive(Deserialize, PartialEq, Debug, Default)]
        struct S {
            other: String,
            #[serde(default)]
            value: Option<String>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.value, None);
    }

    #[test]
    fn test_option_some_number() {
        let ccl = "port = 8080";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            port: Option<u16>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.port, Some(8080));
    }

    // ===========================================
    // Sequence Types (Vec)
    // ===========================================

    #[test]
    fn test_vec_strings_duplicate_keys() {
        // CCL represents lists as duplicate keys
        let ccl = "items = apple\nitems = banana\nitems = cherry";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            items: Vec<String>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.items.len(), 3);
        assert!(s.items.contains(&"apple".to_string()));
        assert!(s.items.contains(&"banana".to_string()));
        assert!(s.items.contains(&"cherry".to_string()));
    }

    #[test]
    fn test_vec_single_item_limitation() {
        // Known limitation: A single value is not distinguishable from a scalar
        // in CCL's key-value model. To get a single-item list, you need the
        // explicit list syntax or use duplicate keys.
        let ccl = "items = only_one";
        #[derive(Deserialize, Debug)]
        struct S {
            items: Vec<String>,
        }
        let result: Result<S> = from_str(ccl);
        // This currently fails because a single value looks like a scalar, not a list
        assert!(result.is_err());
    }

    #[test]
    fn test_vec_two_items() {
        // Two or more duplicate keys correctly become a list
        let ccl = "items = first\nitems = second";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            items: Vec<String>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.items.len(), 2);
        assert!(s.items.contains(&"first".to_string()));
        assert!(s.items.contains(&"second".to_string()));
    }

    // ===========================================
    // Map Types (HashMap)
    // ===========================================

    #[test]
    fn test_hashmap_string_string() {
        let ccl = "env =\n  HOME = /home/user\n  PATH = /usr/bin";
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            env: HashMap<String, String>,
        }
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.env.get("HOME"), Some(&"/home/user".to_string()));
        assert_eq!(s.env.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn test_hashmap_top_level() {
        let ccl = "key1 = value1\nkey2 = value2";
        let map: HashMap<String, String> = from_str(ccl).unwrap();
        assert_eq!(map.get("key1"), Some(&"value1".to_string()));
        assert_eq!(map.get("key2"), Some(&"value2".to_string()));
    }

    // ===========================================
    // Struct Types
    // ===========================================

    #[test]
    fn test_nested_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Inner {
            host: String,
            port: u16,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Outer {
            database: Inner,
        }

        let ccl = "database =\n  host = localhost\n  port = 5432";
        let s: Outer = from_str(ccl).unwrap();
        assert_eq!(s.database.host, "localhost");
        assert_eq!(s.database.port, 5432);
    }

    #[test]
    fn test_deeply_nested_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Level3 {
            value: String,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Level2 {
            level3: Level3,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct Level1 {
            level2: Level2,
        }

        let ccl = "level2 =\n  level3 =\n    value = deep";
        let s: Level1 = from_str(ccl).unwrap();
        assert_eq!(s.level2.level3.value, "deep");
    }

    // ===========================================
    // Enum Types
    // ===========================================

    #[test]
    fn test_unit_enum() {
        #[derive(Deserialize, PartialEq, Debug)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            color: Color,
        }

        let ccl = "color = Red";
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.color, Color::Red);
    }

    #[test]
    fn test_enum_rename_all() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "lowercase")]
        enum Status {
            Active,
            Inactive,
            Pending,
        }
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            status: Status,
        }

        let ccl = "status = active";
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.status, Status::Active);
    }

    // ===========================================
    // Serde Attributes
    // ===========================================

    #[test]
    fn test_rename_field() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            #[serde(rename = "user-name")]
            user_name: String,
        }

        let ccl = "user-name = alice";
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.user_name, "alice");
    }

    #[test]
    fn test_default_value() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            required: String,
            #[serde(default)]
            optional: String,
        }

        let ccl = "required = present";
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.required, "present");
        assert_eq!(s.optional, "");
    }

    #[test]
    fn test_default_with_function() {
        fn default_port() -> u16 {
            3000
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct S {
            host: String,
            #[serde(default = "default_port")]
            port: u16,
        }

        let ccl = "host = localhost";
        let s: S = from_str(ccl).unwrap();
        assert_eq!(s.host, "localhost");
        assert_eq!(s.port, 3000);
    }

    // ===========================================
    // Complex/Combined Types
    // ===========================================

    #[test]
    fn test_vec_of_structs_not_supported() {
        // Note: This documents current limitation - Vec<Struct> may not work
        // as expected in CCL since it's fundamentally a key-value format
        #[derive(Deserialize, PartialEq, Debug)]
        struct Item {
            name: String,
        }
        #[derive(Deserialize, Debug)]
        struct S {
            #[serde(default)]
            items: Vec<Item>,
        }

        // This may or may not work depending on CCL representation
        // Just verify it doesn't panic
        let ccl = "other = something";
        let result: Result<S> = from_str(ccl);
        // Accept either success with empty vec or error
        match result {
            Ok(s) => assert!(s.items.is_empty()),
            Err(_) => {} // Also acceptable
        }
    }

    #[test]
    fn test_hashmap_with_struct_values() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct ServerConfig {
            host: String,
            port: u16,
        }

        let ccl = r#"
web =
  host = localhost
  port = 8080
api =
  host = api.example.com
  port = 443
"#;
        let servers: HashMap<String, ServerConfig> = from_str(ccl).unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers["web"].host, "localhost");
        assert_eq!(servers["web"].port, 8080);
        assert_eq!(servers["api"].host, "api.example.com");
        assert_eq!(servers["api"].port, 443);
    }

    #[test]
    fn test_option_nested_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Database {
            url: String,
        }
        #[derive(Deserialize, PartialEq, Debug, Default)]
        struct Config {
            name: String,
            #[serde(default)]
            database: Option<Database>,
        }

        // With database
        let ccl1 = "name = app\ndatabase =\n  url = postgres://localhost";
        let c1: Config = from_str(ccl1).unwrap();
        assert_eq!(c1.database.as_ref().unwrap().url, "postgres://localhost");

        // Without database
        let ccl2 = "name = app";
        let c2: Config = from_str(ccl2).unwrap();
        assert!(c2.database.is_none());
    }

    // ===========================================
    // Error Cases
    // ===========================================

    #[test]
    fn test_invalid_number_format() {
        #[derive(Deserialize, Debug)]
        struct S {
            port: u16,
        }

        let ccl = "port = not_a_number";
        let result: Result<S> = from_str(ccl);
        assert!(result.is_err());
    }

    #[test]
    fn test_number_overflow() {
        #[derive(Deserialize, Debug)]
        struct S {
            value: u8,
        }

        let ccl = "value = 256"; // u8 max is 255
        let result: Result<S> = from_str(ccl);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_bool() {
        #[derive(Deserialize, Debug)]
        struct S {
            enabled: bool,
        }

        let ccl = "enabled = yes"; // Should be "true" or "false"
        let result: Result<S> = from_str(ccl);
        assert!(result.is_err());
    }

    // ===========================================
    // Real-world Santa Config Types
    // ===========================================

    #[test]
    fn test_source_definition_like_struct() {
        #[derive(Deserialize, Debug)]
        struct SourceDef {
            emoji: String,
            install: String,
            check: String,
            #[serde(default)]
            prefix: Option<String>,
        }

        let ccl = r#"
emoji = 🍺
install = brew install {package}
check = brew leaves --installed-on-request
"#;
        let s: SourceDef = from_str(ccl).unwrap();
        assert_eq!(s.emoji, "🍺");
        assert_eq!(s.install, "brew install {package}");
        assert_eq!(s.check, "brew leaves --installed-on-request");
        assert!(s.prefix.is_none());
    }

    #[test]
    fn test_source_definition_with_prefix() {
        #[derive(Deserialize, Debug)]
        struct SourceDef {
            emoji: String,
            install: String,
            check: String,
            #[serde(default)]
            prefix: Option<String>,
        }

        let ccl = r#"
emoji = ❄️
install = nix-env -iA {package}
check = nix-env -q
prefix = nixpkgs.
"#;
        let s: SourceDef = from_str(ccl).unwrap();
        assert_eq!(s.prefix, Some("nixpkgs.".to_string()));
    }
}

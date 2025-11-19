//! Serde deserialization support for CCL
//!
//! This module provides Serde integration, allowing CCL to be deserialized
//! into Rust structs using the standard `#[derive(Deserialize)]` pattern.

use crate::{Error, Model, Result};
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
pub fn from_model<'de, T>(model: Model) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::from_model(model);
    T::deserialize(&mut deserializer).map_err(|e| Error::ValueError(e.to_string()))
}

/// A structure that deserializes CCL into Rust values
pub struct Deserializer {
    model: Model,
}

impl Deserializer {
    /// Create a new deserializer from a Model
    pub fn from_model(model: Model) -> Self {
        Deserializer { model }
    }
}

/// Helper to extract a string value from the recursive map structure
/// Converts our Result type to DeError for serde compatibility
fn extract_string_value(model: &Model) -> std::result::Result<&str, DeError> {
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
                if value.len() > 1 && value.values().all(|v| v.is_empty()) {
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
            // Special case: if there's only one empty key and its value has entries,
            // use those entries as the list
            if self.model.len() == 1 {
                if let Ok(value) = self.model.get("") {
                    if !value.is_empty() && value.values().all(|v| v.is_empty()) {
                        let items: Vec<String> = value.keys().cloned().collect();
                        let seq = StringSeqDeserializer {
                            iter: items.into_iter(),
                        };
                        return visitor.visit_seq(seq);
                    }
                }
            }

            // Otherwise, extract values with empty keys as a list
            let list: Vec<crate::Model> = self
                .model
                .iter()
                .filter_map(|(k, v)| if k.is_empty() { Some(v.clone()) } else { None })
                .collect();
            let seq = ModelSeqDeserializer {
                iter: list.into_iter(),
            };
            return visitor.visit_seq(seq);
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
    iter: std::vec::IntoIter<Model>,
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
                let model = crate::Model::from_string(s.clone());
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
    iter: indexmap::map::Iter<'a, String, Model>,
    value: Option<&'a Model>,
}

impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a> {
    type Error = DeError;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
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

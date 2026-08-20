//! Serde deserialization for CCL.
//!
//! Deserialization runs over the same [`Item`] tree the editing API uses, so
//! parser behavior — spacing, delimiters, tabs, CRLF, boolean strictness — is
//! configured through [`Options`] exactly as it is elsewhere.
//!
//! ```
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     name: String,
//!     hosts: Vec<String>,
//! }
//!
//! let config: Config = sickle::de::from_str("name = api\nhosts =\n  = a\n  = b\n").unwrap();
//! assert_eq!(config.name, "api");
//! assert_eq!(config.hosts, vec!["a", "b"]);
//! ```

use crate::error::{DeserializeError, Error, Result};
use crate::item::Item;
use crate::options::Options;
use crate::DocumentMut;
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
};
use serde::Deserialize;

/// Deserialize CCL text into `T` using default behaviors.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_str_with(s, &Options::new())
}

/// Deserialize CCL text into `T` using explicit behaviors.
pub fn from_str_with<'a, T>(s: &'a str, options: &Options) -> Result<T>
where
    T: Deserialize<'a>,
{
    let document = DocumentMut::parse_with(s, options)?;
    from_document(&document)
}

/// Deserialize an already-parsed document into `T`.
pub fn from_document<'a, T>(document: &DocumentMut) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_item_with(
        &Item::Table(document.as_table().clone()),
        document.options(),
    )
}

/// Deserialize a single tree node into `T` using default behaviors.
pub fn from_item<'a, T>(item: &Item) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_item_with(item, &Options::new())
}

/// Deserialize a single tree node into `T` using explicit behaviors.
pub fn from_item_with<'a, T>(item: &Item, options: &Options) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::new(item.clone(), options.clone());
    T::deserialize(&mut deserializer).map_err(Error::Deserialize)
}

/// Deserialize a tree node into an owned value.
pub fn from_item_owned<T: DeserializeOwned>(item: &Item, options: &Options) -> Result<T> {
    from_item_with(item, options)
}

/// A Serde deserializer over one node of a CCL tree.
pub struct Deserializer {
    item: Item,
    options: Options,
}

impl Deserializer {
    /// Create a deserializer for `item`.
    pub fn new(item: Item, options: Options) -> Self {
        Self { item, options }
    }

    fn scalar(&self) -> std::result::Result<&str, DeserializeError> {
        self.item.as_str().ok_or_else(|| {
            DeserializeError::new(format!(
                "expected a scalar value, found {}",
                self.item.type_name()
            ))
        })
    }
}

macro_rules! deserialize_number {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let text = self.scalar()?;
            let parsed = text.trim().parse::<$ty>().map_err(|_| {
                DeserializeError::new(format!(
                    concat!("failed to parse '{}' as ", stringify!($ty)),
                    text
                ))
            })?;
            visitor.$visit(parsed)
        }
    };
}

impl<'de> de::Deserializer<'de> for &mut Deserializer {
    type Error = DeserializeError;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.item {
            Item::Value(value) => visitor.visit_str(value.as_str()),
            Item::Array(_) => self.deserialize_seq(visitor),
            Item::Table(table) if table.is_bare_list() => self.deserialize_seq(visitor),
            Item::Table(_) => self.deserialize_map(visitor),
            Item::None => visitor.visit_unit(),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let text = self.scalar()?;
        let parsed = self
            .options
            .parse_bool(text.trim())
            .ok_or_else(|| DeserializeError::new(format!("failed to parse '{text}' as bool")))?;
        visitor.visit_bool(parsed)
    }

    deserialize_number!(deserialize_i8, visit_i8, i8);
    deserialize_number!(deserialize_i16, visit_i16, i16);
    deserialize_number!(deserialize_i32, visit_i32, i32);
    deserialize_number!(deserialize_i64, visit_i64, i64);
    deserialize_number!(deserialize_u8, visit_u8, u8);
    deserialize_number!(deserialize_u16, visit_u16, u16);
    deserialize_number!(deserialize_u32, visit_u32, u32);
    deserialize_number!(deserialize_u64, visit_u64, u64);
    deserialize_number!(deserialize_f32, visit_f32, f32);
    deserialize_number!(deserialize_f64, visit_f64, f64);

    fn deserialize_char<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let text = self.scalar()?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(DeserializeError::new(format!(
                "expected a single character, found '{text}'"
            ))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.scalar()?)
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
        visitor.visit_bytes(self.scalar()?.as_bytes())
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
        if self.item.is_none() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
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
        let items: Vec<Item> = match &self.item {
            Item::Array(array) => array.iter().cloned().collect(),
            Item::Table(table) if table.is_bare_list() => table.values().cloned().collect(),
            Item::Value(value) if value.as_str().is_empty() => Vec::new(),
            Item::Value(value) if contains_bare_list_lines(value.as_str()) => value
                .as_str()
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    trimmed.strip_prefix('=').and_then(|rest| {
                        let text = rest.trim();
                        (!text.is_empty()).then(|| Item::from(text))
                    })
                })
                .collect(),
            // A lone scalar deserializes as a one-element sequence, mirroring
            // CCL's list-coercion behavior.
            Item::Value(value) => vec![Item::Value(value.clone())],
            Item::Table(table) => {
                return Err(DeserializeError::new(format!(
                    "expected a list, found a block with {} entries",
                    table.len()
                )))
            }
            Item::None => Vec::new(),
        };

        visitor.visit_seq(SeqDeserializer {
            iter: items.into_iter(),
            options: self.options.clone(),
        })
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
        let table = self.item.as_table().ok_or_else(|| {
            DeserializeError::new(format!("expected a block, found {}", self.item.type_name()))
        })?;

        let entries: Vec<(String, Item)> = table
            .unique_keys()
            .into_iter()
            .map(|key| {
                (
                    key.to_string(),
                    table.get_composed(key).unwrap_or(Item::None),
                )
            })
            .collect();

        visitor.visit_map(TableDeserializer {
            iter: entries.into_iter(),
            value: None,
            options: self.options.clone(),
        })
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
        visitor.visit_enum(self.scalar()?.into_deserializer())
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

/// Whether a scalar's text contains CCL bare-list lines (`= item`).
fn contains_bare_list_lines(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("= ") || trimmed == "="
    })
}

struct SeqDeserializer {
    iter: std::vec::IntoIter<Item>,
    options: Options,
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = DeserializeError;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(item) => {
                let mut de = Deserializer::new(item, self.options.clone());
                seed.deserialize(&mut de).map(Some)
            }
            None => Ok(None),
        }
    }
}

struct TableDeserializer {
    iter: std::vec::IntoIter<(String, Item)>,
    value: Option<Item>,
    options: Options,
}

impl<'de> MapAccess<'de> for TableDeserializer {
    type Error = DeserializeError;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let item = self
            .value
            .take()
            .ok_or_else(|| DeserializeError::new("value is missing"))?;
        let mut de = Deserializer::new(item, self.options.clone());
        seed.deserialize(&mut de)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Server {
        host: String,
        port: u16,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Config {
        name: String,
        server: Server,
        hosts: Vec<String>,
        #[serde(default)]
        debug: Option<bool>,
    }

    #[test]
    fn deserializes_nested_structs_and_lists() {
        let ccl =
            "name = app\nserver =\n  host = localhost\n  port = 8080\nhosts =\n  = a\n  = b\n";
        let config: Config = from_str(ccl).unwrap();
        assert_eq!(config.name, "app");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.hosts, vec!["a", "b"]);
        assert_eq!(config.debug, None);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let ccl = "/= top\n\nname = app\n/= about the port\nport = 1\n";
        let map: BTreeMap<String, String> = from_str(ccl).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["name"], "app");
    }

    #[test]
    fn repeated_keys_become_sequences() {
        #[derive(Deserialize)]
        struct Doc {
            item: Vec<String>,
        }
        let doc: Doc = from_str("item = a\nitem = b\n").unwrap();
        assert_eq!(doc.item, vec!["a", "b"]);
    }

    #[test]
    fn booleans_follow_configured_strictness() {
        #[derive(Deserialize)]
        struct Doc {
            enabled: bool,
        }
        assert!(from_str::<Doc>("enabled = yes\n").is_err());
        let lenient = Options::new().with_bool(crate::BoolBehavior::Lenient);
        let doc: Doc = from_str_with("enabled = yes\n", &lenient).unwrap();
        assert!(doc.enabled);
    }

    #[test]
    fn empty_values_deserialize_as_empty_lists() {
        #[derive(Deserialize)]
        struct Doc {
            items: Vec<String>,
        }
        let doc: Doc = from_str("items =\n").unwrap();
        assert!(doc.items.is_empty());
    }

    #[test]
    fn lists_of_records_are_supported() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Item2 {
            name: String,
        }
        #[derive(Debug, Deserialize, PartialEq)]
        struct Doc {
            items: Vec<Item2>,
        }
        let doc: Doc = from_str("items =\n  =\n    name = a\n  =\n    name = b\n").unwrap();
        assert_eq!(doc.items.len(), 2);
        assert_eq!(doc.items[0].name, "a");
    }

    #[test]
    fn enums_deserialize_from_scalars() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(rename_all = "lowercase")]
        enum Mode {
            Fast,
            Slow,
        }
        #[derive(Debug, Deserialize, PartialEq)]
        struct Doc {
            mode: Mode,
        }
        let doc: Doc = from_str("mode = fast\n").unwrap();
        assert_eq!(doc.mode, Mode::Fast);
    }

    #[test]
    fn missing_fields_report_errors() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Doc {
            required: String,
        }
        let err = from_str::<Doc>("other = 1\n").unwrap_err();
        assert!(matches!(err, Error::Deserialize(_)));
    }
}

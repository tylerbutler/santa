//! Serde serialization to CCL.
//!
//! Serialization builds an [`Item`] tree and renders it in canonical form: two
//! space indentation, `key = value` for scalars, `key =` plus an indented block
//! for nested data, and `= item` lines for lists.
//!
//! Serializing is lossy with respect to formatting — it produces new text, so
//! there are no comments to preserve. To keep a hand-written file's comments,
//! parse it into a [`DocumentMut`] and write individual
//! values back through the checked editing API instead.
//!
//! ```
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Config {
//!     name: String,
//!     hosts: Vec<String>,
//! }
//!
//! let config = Config { name: "api".into(), hosts: vec!["a".into(), "b".into()] };
//! assert_eq!(
//!     sickle::ser::to_string(&config).unwrap(),
//!     "name = api\nhosts =\n  = a\n  = b"
//! );
//! ```

use crate::error::{Error, Result, SerializeError};
use crate::item::Item;
use crate::repr::Key;
use crate::table::{Array, Table};
use crate::value::Value;
use crate::DocumentMut;
use serde::ser::{self, Serialize};

/// Serialize `value` to canonical CCL text.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize + ?Sized,
{
    let item = to_item(value)?;
    Ok(crate::encode::render_document(&as_root(item)))
}

/// Serialize `value` into a [`DocumentMut`] that can be edited further.
pub fn to_document<T>(value: &T) -> Result<DocumentMut>
where
    T: Serialize + ?Sized,
{
    Ok(DocumentMut::from(as_root(to_item(value)?)))
}

/// Serialize `value` into a tree node.
pub fn to_item<T>(value: &T) -> Result<Item>
where
    T: Serialize + ?Sized,
{
    value.serialize(Serializer).map_err(Error::Serialize)
}

/// Wrap a serialized node so it can be rendered as a document body.
fn as_root(item: Item) -> Table {
    match item {
        Item::Table(table) => table,
        Item::None => Table::new(),
        Item::Array(array) => {
            let mut table = Table::new();
            for element in array.iter() {
                table.append(Key::new(""), element.clone());
            }
            table
        }
        Item::Value(value) => {
            let mut table = Table::new();
            table.append(Key::new(""), Item::Value(value));
            table
        }
    }
}

/// A Serde serializer producing CCL tree nodes.
pub struct Serializer;

type SerResult = std::result::Result<Item, SerializeError>;

impl ser::Serializer for Serializer {
    type Ok = Item;
    type Error = SerializeError;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = VariantSeqSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = VariantMapSerializer;

    fn serialize_bool(self, v: bool) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_i8(self, v: i8) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_i16(self, v: i16) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_i32(self, v: i32) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_i64(self, v: i64) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_u8(self, v: u8) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_u16(self, v: u16) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_u32(self, v: u32) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_u64(self, v: u64) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_f32(self, v: f32) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_f64(self, v: f64) -> SerResult {
        Ok(Item::Value(Value::from(v)))
    }

    fn serialize_char(self, v: char) -> SerResult {
        Ok(Item::Value(Value::new(v.to_string())))
    }

    fn serialize_str(self, v: &str) -> SerResult {
        Ok(Item::Value(Value::new(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> SerResult {
        Ok(Item::Value(Value::new(String::from_utf8_lossy(v))))
    }

    fn serialize_none(self) -> SerResult {
        Ok(Item::None)
    }

    fn serialize_some<T>(self, value: &T) -> SerResult
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> SerResult {
        Ok(Item::None)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> SerResult {
        Ok(Item::None)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> SerResult {
        Ok(Item::Value(Value::new(variant)))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> SerResult
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> SerResult
    where
        T: ?Sized + Serialize,
    {
        let inner = value.serialize(Serializer)?;
        let mut table = Table::new();
        table.append(Key::new(variant), inner);
        Ok(Item::Table(table))
    }

    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            array: Array::new(),
        })
    }

    fn serialize_tuple(
        self,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer {
            array: Array::new(),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SeqSerializer {
            array: Array::new(),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(VariantSeqSerializer {
            variant,
            array: Array::new(),
        })
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            table: Table::new(),
            key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        Ok(MapSerializer {
            table: Table::new(),
            key: None,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Ok(VariantMapSerializer {
            variant,
            table: Table::new(),
        })
    }
}

/// Serializes sequences into a bare-syntax [`Array`].
pub struct SeqSerializer {
    array: Array,
}

impl SeqSerializer {
    fn push<T>(&mut self, value: &T) -> std::result::Result<(), SerializeError>
    where
        T: ?Sized + Serialize,
    {
        let item = value.serialize(Serializer)?;
        if !item.is_none() {
            self.array.push(item);
        }
        Ok(())
    }
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.push(value)
    }

    fn end(self) -> SerResult {
        Ok(Item::Array(self.array))
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.push(value)
    }

    fn end(self) -> SerResult {
        Ok(Item::Array(self.array))
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.push(value)
    }

    fn end(self) -> SerResult {
        Ok(Item::Array(self.array))
    }
}

/// Serializes tuple variants into `variant =` followed by a list.
pub struct VariantSeqSerializer {
    variant: &'static str,
    array: Array,
}

impl ser::SerializeTupleVariant for VariantSeqSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let item = value.serialize(Serializer)?;
        if !item.is_none() {
            self.array.push(item);
        }
        Ok(())
    }

    fn end(self) -> SerResult {
        let mut table = Table::new();
        table.append(Key::new(self.variant), Item::Array(self.array));
        Ok(Item::Table(table))
    }
}

/// Serializes maps and structs into a [`Table`].
pub struct MapSerializer {
    table: Table,
    key: Option<String>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_key<T>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let item = key.serialize(Serializer)?;
        let text = item.as_str().ok_or_else(|| {
            SerializeError::new("CCL map keys must serialize to a scalar".to_string())
        })?;
        self.key = Some(text.to_string());
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| SerializeError::new("map value serialized before its key"))?;
        let item = value.serialize(Serializer)?;
        if !item.is_none() {
            self.table.append(Key::new(key), item);
        }
        Ok(())
    }

    fn end(self) -> SerResult {
        Ok(Item::Table(self.table))
    }
}

impl ser::SerializeStruct for MapSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let item = value.serialize(Serializer)?;
        if !item.is_none() {
            self.table.append(Key::new(key), item);
        }
        Ok(())
    }

    fn end(self) -> SerResult {
        Ok(Item::Table(self.table))
    }
}

/// Serializes struct variants into `variant =` followed by a block.
pub struct VariantMapSerializer {
    variant: &'static str,
    table: Table,
}

impl ser::SerializeStructVariant for VariantMapSerializer {
    type Ok = Item;
    type Error = SerializeError;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let item = value.serialize(Serializer)?;
        if !item.is_none() {
            self.table.append(Key::new(key), item);
        }
        Ok(())
    }

    fn end(self) -> SerResult {
        let mut table = Table::new();
        table.append(Key::new(self.variant), Item::Table(self.table));
        Ok(Item::Table(table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Server {
        host: String,
        port: u16,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Config {
        name: String,
        server: Server,
        hosts: Vec<String>,
        enabled: bool,
    }

    #[test]
    fn serializes_nested_structures_canonically() {
        let config = Config {
            name: "app".into(),
            server: Server {
                host: "localhost".into(),
                port: 8080,
            },
            hosts: vec!["a".into(), "b".into()],
            enabled: true,
        };
        assert_eq!(
            to_string(&config).unwrap(),
            "name = app\nserver =\n  host = localhost\n  port = 8080\nhosts =\n  = a\n  = b\nenabled = true"
        );
    }

    #[test]
    fn round_trips_through_deserialization() {
        let config = Config {
            name: "app".into(),
            server: Server {
                host: "h".into(),
                port: 1,
            },
            hosts: vec!["x".into()],
            enabled: false,
        };
        let text = to_string(&config).unwrap();
        let back: Config = crate::de::from_str(&text).unwrap();
        assert_eq!(config, back);
    }

    #[test]
    fn none_fields_are_omitted() {
        #[derive(Serialize)]
        struct Doc {
            present: String,
            absent: Option<String>,
        }
        let text = to_string(&Doc {
            present: "1".into(),
            absent: None,
        })
        .unwrap();
        assert_eq!(text, "present = 1");
    }

    #[test]
    fn maps_serialize_in_iteration_order() {
        let mut map = BTreeMap::new();
        map.insert("b", 2);
        map.insert("a", 1);
        assert_eq!(to_string(&map).unwrap(), "a = 1\nb = 2");
    }

    #[test]
    fn to_document_allows_further_editing() {
        let mut map = BTreeMap::new();
        map.insert("a", 1);
        let mut doc = to_document(&map).unwrap();
        doc.insert_comment_before(["a"], "generated").unwrap();
        assert_eq!(doc.to_string(), "/= generated\na = 1");
    }

    #[test]
    fn enum_variants_serialize_as_scalars() {
        #[derive(Serialize)]
        #[serde(rename_all = "lowercase")]
        enum Mode {
            Fast,
        }
        #[derive(Serialize)]
        struct Doc {
            mode: Mode,
        }
        assert_eq!(to_string(&Doc { mode: Mode::Fast }).unwrap(), "mode = fast");
    }

    #[test]
    fn lists_of_records_use_bare_syntax() {
        #[derive(Serialize)]
        struct Row {
            name: String,
        }
        #[derive(Serialize)]
        struct Doc {
            rows: Vec<Row>,
        }
        let doc = Doc {
            rows: vec![Row { name: "a".into() }, Row { name: "b".into() }],
        };
        assert_eq!(
            to_string(&doc).unwrap(),
            "rows =\n  =\n    name = a\n  =\n    name = b"
        );
    }
}

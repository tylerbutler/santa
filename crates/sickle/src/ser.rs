//! Serde serialization support for CCL
//!
//! This module provides Serde integration, allowing Rust structs to be serialized
//! into CCL format using the standard `#[derive(Serialize)]` pattern.

use crate::printer::{CclPrinter, PrinterConfig};
use crate::{CclObject, Result};
use serde::ser::{self, Serialize};
use std::fmt;

/// Serialize a value to a CCL string
///
/// # Examples
///
/// ```rust
/// use serde::Serialize;
/// use sickle::to_string;
///
/// #[derive(Serialize)]
/// struct Config {
///     name: String,
///     version: String,
/// }
///
/// let config = Config {
///     name: "MyApp".to_string(),
///     version: "1.0.0".to_string(),
/// };
///
/// let ccl = to_string(&config).unwrap();
/// assert!(ccl.contains("name = MyApp"));
/// assert!(ccl.contains("version = 1.0.0"));
/// ```
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    to_string_with_config(value, PrinterConfig::default())
}

/// Serialize a value to a CCL string with custom printer configuration
pub fn to_string_with_config<T>(value: &T, config: PrinterConfig) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer::new();
    value.serialize(&mut serializer)?;
    let model = serializer.into_model();
    let printer = CclPrinter::with_config(config);
    Ok(printer.print(&model))
}

/// A structure that serializes Rust values into CCL
pub struct Serializer {
    /// The current key being serialized (for map entries)
    current_key: Option<String>,
    /// Stack of nested objects being built
    stack: Vec<CclObject>,
}

impl Serializer {
    /// Create a new serializer
    pub fn new() -> Self {
        Serializer {
            current_key: None,
            stack: vec![CclObject::new()],
        }
    }

    /// Get the resulting model after serialization
    pub fn into_model(mut self) -> CclObject {
        self.stack.pop().unwrap_or_else(CclObject::new)
    }

    /// Get mutable reference to the current object being built
    fn current_object(&mut self) -> &mut CclObject {
        self.stack.last_mut().expect("stack should not be empty")
    }

    /// Insert a string value at the current position
    fn insert_value(&mut self, value: String) {
        if let Some(key) = self.current_key.take() {
            self.current_object().insert_string(&key, value);
        }
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom error type for serialization
#[derive(Debug, Clone)]
pub struct SerError {
    msg: String,
}

impl SerError {
    fn custom(msg: impl fmt::Display) -> Self {
        SerError {
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for SerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for SerError {}

impl ser::Error for SerError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerError::custom(msg)
    }
}

impl From<SerError> for crate::Error {
    fn from(e: SerError) -> Self {
        crate::Error::ValueError(e.msg)
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = SerError;

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = SeqSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = MapSerializer<'a>;

    fn serialize_bool(self, v: bool) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_char(self, v: char) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_str(self, v: &str) -> std::result::Result<(), Self::Error> {
        self.insert_value(v.to_string());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> std::result::Result<(), Self::Error> {
        // Convert bytes to a string representation
        self.insert_value(String::from_utf8_lossy(v).to_string());
        Ok(())
    }

    fn serialize_none(self) -> std::result::Result<(), Self::Error> {
        // None values are simply not serialized (key is dropped)
        self.current_key = None;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> std::result::Result<(), Self::Error> {
        self.insert_value(variant.to_string());
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        // Store the variant name as the key
        let key = self.current_key.take();
        if let Some(k) = key {
            self.current_key = Some(k);
        }
        self.current_key = Some(variant.to_string());
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
        })
    }

    fn serialize_tuple(self, _len: usize) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> std::result::Result<Self::SerializeMap, Self::Error> {
        // Save the current key (if any) as the parent key for this nested map
        let parent_key = self.current_key.take();
        // Push a new object onto the stack for nested maps
        self.stack.push(CclObject::new());
        Ok(MapSerializer { ser: self, parent_key })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        // Save the current key (if any) as the parent key for this nested struct
        let parent_key = self.current_key.take();
        // Push a new object onto the stack for nested structs
        self.stack.push(CclObject::new());
        Ok(MapSerializer { ser: self, parent_key })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        let parent_key = self.current_key.take();
        self.stack.push(CclObject::new());
        Ok(MapSerializer { ser: self, parent_key })
    }
}

/// Serializer for sequences (Vec, arrays, etc.)
pub struct SeqSerializer<'a> {
    ser: &'a mut Serializer,
    items: Vec<String>,
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        // Serialize the element to a string
        let mut element_ser = Serializer::new();
        element_ser.current_key = Some(String::new());
        value.serialize(&mut element_ser)?;

        // Extract the value from the serialized result
        let model = element_ser.into_model();
        if let Some((_, inner)) = model.iter().next() {
            if let Some((val, _)) = inner.iter().next() {
                self.items.push(val.clone());
            }
        }
        Ok(())
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        if let Some(key) = self.ser.current_key.take() {
            self.ser.current_object().insert_list(&key, self.items);
        }
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializer for maps and structs
pub struct MapSerializer<'a> {
    ser: &'a mut Serializer,
    /// The key under which this map/struct will be inserted (for nested structs)
    parent_key: Option<String>,
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        // Serialize the key to a string
        let mut key_ser = Serializer::new();
        key_ser.current_key = Some(String::new());
        key.serialize(&mut key_ser)?;

        let model = key_ser.into_model();
        if let Some((_, inner)) = model.iter().next() {
            if let Some((key_str, _)) = inner.iter().next() {
                self.ser.current_key = Some(key_str.clone());
            }
        }
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        // Pop the completed object and merge it into the parent
        if self.ser.stack.len() > 1 {
            let completed = self.ser.stack.pop().unwrap();
            if let Some(key) = self.parent_key {
                // Nested map/struct: insert under the parent key
                self.ser.current_object().insert_object(&key, completed);
            } else {
                // Top-level map, merge into current
                let current = self.ser.current_object();
                for (k, v) in completed.iter() {
                    current.insert_object(k, v.clone());
                }
            }
        }
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for MapSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.ser.current_key = Some(key.to_string());
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeMap::end(self)
    }
}

impl<'a> ser::SerializeStructVariant for MapSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeMap::end(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn test_serialize_simple_struct() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            version: String,
        }

        let config = Config {
            name: "MyApp".to_string(),
            version: "1.0.0".to_string(),
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("name = MyApp"));
        assert!(ccl.contains("version = 1.0.0"));
    }

    #[test]
    fn test_serialize_with_numbers() {
        #[derive(Serialize)]
        struct Config {
            port: u16,
            timeout: u32,
            enabled: bool,
        }

        let config = Config {
            port: 8080,
            timeout: 3000,
            enabled: true,
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("port = 8080"));
        assert!(ccl.contains("timeout = 3000"));
        assert!(ccl.contains("enabled = true"));
    }

    #[test]
    fn test_serialize_nested_struct() {
        #[derive(Serialize)]
        struct Database {
            host: String,
            port: u16,
        }

        #[derive(Serialize)]
        struct Config {
            name: String,
            database: Database,
        }

        let config = Config {
            name: "MyApp".to_string(),
            database: Database {
                host: "localhost".to_string(),
                port: 5432,
            },
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("name = MyApp"));
        assert!(ccl.contains("database ="));
        assert!(ccl.contains("host = localhost"));
        assert!(ccl.contains("port = 5432"));
    }

    #[test]
    fn test_serialize_option_some() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            description: Option<String>,
        }

        let config = Config {
            name: "MyApp".to_string(),
            description: Some("A great app".to_string()),
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("name = MyApp"));
        assert!(ccl.contains("description = A great app"));
    }

    #[test]
    fn test_serialize_option_none() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            description: Option<String>,
        }

        let config = Config {
            name: "MyApp".to_string(),
            description: None,
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("name = MyApp"));
        assert!(!ccl.contains("description"));
    }

    #[test]
    fn test_serialize_enum() {
        #[derive(Serialize)]
        enum Status {
            Active,
            Inactive,
        }

        #[derive(Serialize)]
        struct Config {
            status: Status,
        }

        let config = Config {
            status: Status::Active,
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("status = Active"));
    }

    #[test]
    fn test_serialize_vec() {
        #[derive(Serialize)]
        struct Config {
            tags: Vec<String>,
        }

        let config = Config {
            tags: vec!["rust".to_string(), "ccl".to_string(), "parser".to_string()],
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("tags ="));
        assert!(ccl.contains("rust"));
        assert!(ccl.contains("ccl"));
        assert!(ccl.contains("parser"));
    }

    #[test]
    fn test_serialize_hashmap() {
        use std::collections::HashMap;

        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("HOME".to_string(), "/home/user".to_string());
        env.insert("PATH".to_string(), "/usr/bin".to_string());

        let ccl = to_string(&env).unwrap();
        assert!(ccl.contains("HOME = /home/user"));
        assert!(ccl.contains("PATH = /usr/bin"));
    }

    #[test]
    fn test_serialize_floats() {
        #[derive(Serialize)]
        struct Config {
            ratio: f64,
            scale: f32,
        }

        let config = Config {
            ratio: 3.14159,
            scale: 2.5,
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("ratio = 3.14159"));
        assert!(ccl.contains("scale = 2.5"));
    }

    #[test]
    fn test_serialize_deeply_nested() {
        #[derive(Serialize)]
        struct Level3 {
            value: String,
        }

        #[derive(Serialize)]
        struct Level2 {
            level3: Level3,
        }

        #[derive(Serialize)]
        struct Level1 {
            level2: Level2,
        }

        let config = Level1 {
            level2: Level2 {
                level3: Level3 {
                    value: "deep".to_string(),
                },
            },
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("level2 ="));
        assert!(ccl.contains("level3 ="));
        assert!(ccl.contains("value = deep"));
    }

    #[test]
    fn test_roundtrip_simple() {
        use serde::Deserialize;

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            port: u16,
            enabled: bool,
        }

        let original = Config {
            name: "MyApp".to_string(),
            port: 8080,
            enabled: true,
        };

        let ccl = to_string(&original).unwrap();
        let parsed: Config = crate::from_str(&ccl).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_nested() {
        use serde::Deserialize;

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Database {
            host: String,
            port: u16,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            database: Database,
        }

        let original = Config {
            name: "MyApp".to_string(),
            database: Database {
                host: "localhost".to_string(),
                port: 5432,
            },
        };

        let ccl = to_string(&original).unwrap();
        let parsed: Config = crate::from_str(&ccl).unwrap();

        assert_eq!(original, parsed);
    }
}

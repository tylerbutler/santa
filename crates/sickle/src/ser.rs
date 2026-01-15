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
        self.stack.pop().unwrap_or_default()
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

    fn serialize_some<T>(self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // Store the variant name as the key
        let key = self.current_key.take();
        if let Some(k) = key {
            self.current_key = Some(k);
        }
        self.current_key = Some(variant.to_string());
        value.serialize(self)
    }

    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
            nested_items: Vec::new(),
        })
    }

    fn serialize_tuple(
        self,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer {
            ser: self,
            items: Vec::new(),
            nested_items: Vec::new(),
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
            nested_items: Vec::new(),
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
            nested_items: Vec::new(),
        })
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        // Save the current key (if any) as the parent key for this nested map
        let parent_key = self.current_key.take();
        // Push a new object onto the stack for nested maps
        self.stack.push(CclObject::new());
        Ok(MapSerializer {
            ser: self,
            parent_key,
        })
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
        Ok(MapSerializer {
            ser: self,
            parent_key,
        })
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
        Ok(MapSerializer {
            ser: self,
            parent_key,
        })
    }
}

/// Serializer for sequences (Vec, arrays, etc.)
pub struct SeqSerializer<'a> {
    ser: &'a mut Serializer,
    /// Simple string items (for `Vec<String>`, `Vec<i32>`, etc.)
    items: Vec<String>,
    /// Complex nested items (for `Vec<Struct>`, `Vec<HashMap>`, etc.)
    nested_items: Vec<CclObject>,
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // Serialize the element using a fresh serializer
        let mut element_ser = Serializer::new();
        element_ser.current_key = Some(String::new());
        value.serialize(&mut element_ser)?;

        // Extract the value from the serialized result
        let model = element_ser.into_model();

        // Look for the empty-key entry (where serialize put our value)
        if let Some((_, values)) = model.iter_map().find(|(k, _)| k.is_empty()) {
            if let Some(inner) = values.first() {
                // Check if it's a simple string value: single key with empty children
                if inner.len() == 1 {
                    if let Some((key, children)) = inner.iter_map().next() {
                        if children.iter().all(|c| c.is_empty()) {
                            // Simple string value - add to items list
                            self.items.push(key.clone());
                            return Ok(());
                        }
                    }
                }
                // Complex nested object - add to nested_items
                self.nested_items.push(inner.clone());
            }
        }
        Ok(())
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        if let Some(key) = self.ser.current_key.take() {
            if !self.nested_items.is_empty() {
                // Create list of nested objects using bare list syntax
                // Each nested object becomes a child under an empty key
                let mut list_obj = CclObject::new();
                let map = list_obj.inner_mut();
                map.insert("".to_string(), self.nested_items);
                self.ser.current_object().insert_object(&key, list_obj);
            } else if !self.items.is_empty() {
                self.ser.current_object().insert_list(&key, self.items);
            }
        }
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_field<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_key<T>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_value<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
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

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> std::result::Result<(), Self::Error> {
        ser::SerializeMap::end(self)
    }
}

/// Comprehensive serde validation tests for the CCL serializer.
///
/// These tests verify that the serializer correctly handles all Serde data types
/// by testing actual serialization to CCL strings and round-trip validation.
/// Mirrors the structure of `de::serde_validation_tests` for consistency.
#[cfg(test)]
mod serde_validation_tests {
    #![allow(dead_code)] // Test structs/enums exist to verify serialization, not field usage

    use super::*;
    use crate::printer::PrinterConfig;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    // ===========================================
    // Primitive Types - Signed Integers
    // ===========================================

    #[test]
    fn test_i8() {
        #[derive(Serialize)]
        struct S {
            value: i8,
        }
        let s = S { value: -128 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = -128"));
    }

    #[test]
    fn test_i8_positive() {
        #[derive(Serialize)]
        struct S {
            value: i8,
        }
        let s = S { value: 127 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 127"));
    }

    #[test]
    fn test_i16() {
        #[derive(Serialize)]
        struct S {
            value: i16,
        }
        let s = S { value: -32768 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = -32768"));
    }

    #[test]
    fn test_i32() {
        #[derive(Serialize)]
        struct S {
            value: i32,
        }
        let s = S { value: -2147483648 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = -2147483648"));
    }

    #[test]
    fn test_i64() {
        #[derive(Serialize)]
        struct S {
            value: i64,
        }
        let s = S {
            value: -9223372036854775808,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = -9223372036854775808"));
    }

    // ===========================================
    // Primitive Types - Unsigned Integers
    // ===========================================

    #[test]
    fn test_u8() {
        #[derive(Serialize)]
        struct S {
            value: u8,
        }
        let s = S { value: 255 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 255"));
    }

    #[test]
    fn test_u16() {
        #[derive(Serialize)]
        struct S {
            value: u16,
        }
        let s = S { value: 65535 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 65535"));
    }

    #[test]
    fn test_u32() {
        #[derive(Serialize)]
        struct S {
            value: u32,
        }
        let s = S { value: 4294967295 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 4294967295"));
    }

    #[test]
    fn test_u64() {
        #[derive(Serialize)]
        struct S {
            value: u64,
        }
        let s = S {
            value: 18446744073709551615,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 18446744073709551615"));
    }

    // ===========================================
    // Primitive Types - Floating Point
    // ===========================================

    #[test]
    fn test_f32() {
        #[derive(Serialize)]
        struct S {
            value: f32,
        }
        let s = S { value: 3.14 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 3.14"));
    }

    #[test]
    fn test_f64() {
        #[derive(Serialize)]
        struct S {
            value: f64,
        }
        let s = S {
            value: 3.141592653589793,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 3.141592653589793"));
    }

    #[test]
    fn test_f32_negative() {
        #[derive(Serialize)]
        struct S {
            value: f32,
        }
        let s = S { value: -2.5 };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = -2.5"));
    }

    // ===========================================
    // Primitive Types - Boolean
    // ===========================================

    #[test]
    fn test_bool_true() {
        #[derive(Serialize)]
        struct S {
            value: bool,
        }
        let s = S { value: true };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = true"));
    }

    #[test]
    fn test_bool_false() {
        #[derive(Serialize)]
        struct S {
            value: bool,
        }
        let s = S { value: false };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = false"));
    }

    // ===========================================
    // Primitive Types - Char and String
    // ===========================================

    #[test]
    fn test_char() {
        #[derive(Serialize)]
        struct S {
            value: char,
        }
        let s = S { value: 'X' };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = X"));
    }

    #[test]
    fn test_char_unicode() {
        #[derive(Serialize)]
        struct S {
            value: char,
        }
        let s = S { value: '日' };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 日"));
    }

    #[test]
    fn test_string() {
        #[derive(Serialize)]
        struct S {
            value: String,
        }
        let s = S {
            value: "hello world".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = hello world"));
    }

    #[test]
    fn test_str_ref() {
        #[derive(Serialize)]
        struct S<'a> {
            value: &'a str,
        }
        let s = S { value: "borrowed" };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = borrowed"));
    }

    // ===========================================
    // Primitive Types - Bytes
    // ===========================================

    #[test]
    fn test_bytes() {
        // Test bytes serialization via serde_bytes or manual
        #[derive(Serialize)]
        struct S {
            #[serde(serialize_with = "serialize_bytes")]
            data: Vec<u8>,
        }

        fn serialize_bytes<S: serde::Serializer>(
            bytes: &[u8],
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            serializer.serialize_bytes(bytes)
        }

        let s = S {
            data: vec![72, 101, 108, 108, 111], // "Hello"
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("data = Hello"));
    }

    // ===========================================
    // Option Types
    // ===========================================

    #[test]
    fn test_option_some_string() {
        #[derive(Serialize)]
        struct S {
            value: Option<String>,
        }
        let s = S {
            value: Some("present".to_string()),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = present"));
    }

    #[test]
    fn test_option_none() {
        #[derive(Serialize)]
        struct S {
            name: String,
            value: Option<String>,
        }
        let s = S {
            name: "test".to_string(),
            value: None,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("name = test"));
        assert!(!ccl.contains("value"));
    }

    #[test]
    fn test_option_some_number() {
        #[derive(Serialize)]
        struct S {
            port: Option<u16>,
        }
        let s = S { port: Some(8080) };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("port = 8080"));
    }

    #[test]
    fn test_option_some_bool() {
        #[derive(Serialize)]
        struct S {
            enabled: Option<bool>,
        }
        let s = S {
            enabled: Some(true),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("enabled = true"));
    }

    // ===========================================
    // Unit Types
    // ===========================================

    #[test]
    fn test_unit() {
        // Unit type serializes to nothing
        let ccl = to_string(&()).unwrap();
        assert!(ccl.is_empty() || ccl.trim().is_empty());
    }

    #[test]
    fn test_unit_struct() {
        #[derive(Serialize)]
        struct Unit;

        let ccl = to_string(&Unit).unwrap();
        // Unit struct should serialize to empty or minimal output
        assert!(ccl.is_empty() || ccl.trim().is_empty());
    }

    // ===========================================
    // Newtype Structs
    // ===========================================

    #[test]
    fn test_newtype_struct() {
        #[derive(Serialize)]
        struct Wrapper(String);

        #[derive(Serialize)]
        struct S {
            wrapped: Wrapper,
        }

        let s = S {
            wrapped: Wrapper("inner value".to_string()),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("wrapped = inner value"));
    }

    #[test]
    fn test_newtype_struct_number() {
        #[derive(Serialize)]
        struct Port(u16);

        #[derive(Serialize)]
        struct S {
            port: Port,
        }

        let s = S { port: Port(8080) };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("port = 8080"));
    }

    // ===========================================
    // Tuple Types
    // ===========================================

    #[test]
    fn test_tuple() {
        #[derive(Serialize)]
        struct S {
            point: (i32, i32),
        }

        let s = S { point: (10, 20) };
        let ccl = to_string(&s).unwrap();
        // Tuples serialize as lists
        assert!(ccl.contains("point ="));
        assert!(ccl.contains("10"));
        assert!(ccl.contains("20"));
    }

    #[test]
    fn test_tuple_struct() {
        #[derive(Serialize)]
        struct Point(i32, i32);

        #[derive(Serialize)]
        struct S {
            location: Point,
        }

        let s = S {
            location: Point(100, 200),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("location ="));
        assert!(ccl.contains("100"));
        assert!(ccl.contains("200"));
    }

    #[test]
    fn test_tuple_three_elements() {
        #[derive(Serialize)]
        struct S {
            rgb: (u8, u8, u8),
        }

        let s = S { rgb: (255, 128, 0) };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("rgb ="));
        assert!(ccl.contains("255"));
        assert!(ccl.contains("128"));
        assert!(ccl.contains("0"));
    }

    // ===========================================
    // Sequence Types (Vec)
    // ===========================================

    #[test]
    fn test_vec_strings() {
        #[derive(Serialize)]
        struct S {
            items: Vec<String>,
        }

        let s = S {
            items: vec![
                "apple".to_string(),
                "banana".to_string(),
                "cherry".to_string(),
            ],
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("items ="));
        assert!(ccl.contains("apple"));
        assert!(ccl.contains("banana"));
        assert!(ccl.contains("cherry"));
    }

    #[test]
    fn test_vec_numbers() {
        #[derive(Serialize)]
        struct S {
            values: Vec<i32>,
        }

        let s = S {
            values: vec![1, 2, 3, 4, 5],
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("values ="));
        for n in 1..=5 {
            assert!(ccl.contains(&n.to_string()));
        }
    }

    #[test]
    fn test_vec_empty() {
        #[derive(Serialize)]
        struct S {
            name: String,
            items: Vec<String>,
        }

        let s = S {
            name: "test".to_string(),
            items: vec![],
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("name = test"));
        // Empty vec might not appear or appear as empty
    }

    #[test]
    fn test_vec_single_item() {
        #[derive(Serialize)]
        struct S {
            items: Vec<String>,
        }

        let s = S {
            items: vec!["only".to_string()],
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("only"));
    }

    // ===========================================
    // Map Types (HashMap)
    // ===========================================

    #[test]
    fn test_hashmap_string_string() {
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("HOME".to_string(), "/home/user".to_string());
        map.insert("PATH".to_string(), "/usr/bin".to_string());

        let ccl = to_string(&map).unwrap();
        assert!(ccl.contains("HOME = /home/user"));
        assert!(ccl.contains("PATH = /usr/bin"));
    }

    #[test]
    fn test_hashmap_nested_in_struct() {
        #[derive(Serialize)]
        struct S {
            env: HashMap<String, String>,
        }

        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let s = S { env };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("env ="));
        assert!(ccl.contains("KEY = value"));
    }

    #[test]
    fn test_hashmap_empty() {
        let map: HashMap<String, String> = HashMap::new();
        let ccl = to_string(&map).unwrap();
        // Empty map should produce empty or minimal output
        assert!(ccl.is_empty() || ccl.trim().is_empty());
    }

    #[test]
    fn test_hashmap_with_struct_values() {
        #[derive(Serialize)]
        struct Inner {
            value: i32,
        }

        let mut map: HashMap<String, Inner> = HashMap::new();
        map.insert("first".to_string(), Inner { value: 1 });
        map.insert("second".to_string(), Inner { value: 2 });

        let ccl = to_string(&map).unwrap();
        assert!(ccl.contains("first ="));
        assert!(ccl.contains("second ="));
        assert!(ccl.contains("value = 1") || ccl.contains("value = 2"));
    }

    // ===========================================
    // Struct Types
    // ===========================================

    #[test]
    fn test_simple_struct() {
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
    fn test_nested_struct() {
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
    fn test_deeply_nested_struct() {
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
    fn test_struct_with_multiple_fields() {
        #[derive(Serialize)]
        struct Config {
            name: String,
            port: u16,
            enabled: bool,
            timeout: f64,
        }

        let config = Config {
            name: "server".to_string(),
            port: 8080,
            enabled: true,
            timeout: 30.5,
        };

        let ccl = to_string(&config).unwrap();
        assert!(ccl.contains("name = server"));
        assert!(ccl.contains("port = 8080"));
        assert!(ccl.contains("enabled = true"));
        assert!(ccl.contains("timeout = 30.5"));
    }

    // ===========================================
    // Enum Types
    // ===========================================

    #[test]
    fn test_unit_variant() {
        #[derive(Serialize)]
        enum Status {
            Active,
            Inactive,
            Pending,
        }

        #[derive(Serialize)]
        struct S {
            status: Status,
        }

        let s = S {
            status: Status::Active,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("status = Active"));

        let s2 = S {
            status: Status::Inactive,
        };
        let ccl2 = to_string(&s2).unwrap();
        assert!(ccl2.contains("status = Inactive"));
    }

    #[test]
    fn test_newtype_variant() {
        #[derive(Serialize)]
        enum Value {
            Text(String),
            Number(i32),
        }

        #[derive(Serialize)]
        struct S {
            data: Value,
        }

        let s = S {
            data: Value::Text("hello".to_string()),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("Text = hello") || ccl.contains("hello"));
    }

    #[test]
    fn test_tuple_variant() {
        #[derive(Serialize)]
        enum Point {
            TwoD(i32, i32),
            ThreeD(i32, i32, i32),
        }

        #[derive(Serialize)]
        struct S {
            point: Point,
        }

        let s = S {
            point: Point::TwoD(10, 20),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("10"));
        assert!(ccl.contains("20"));
    }

    #[test]
    fn test_struct_variant() {
        #[derive(Serialize)]
        enum Message {
            Request { id: u32, method: String },
            Response { id: u32, result: String },
        }

        #[derive(Serialize)]
        struct S {
            msg: Message,
        }

        let s = S {
            msg: Message::Request {
                id: 1,
                method: "GET".to_string(),
            },
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("id = 1") || ccl.contains("1"));
        assert!(ccl.contains("method = GET") || ccl.contains("GET"));
    }

    // ===========================================
    // Serde Attributes
    // ===========================================

    #[test]
    fn test_rename_field() {
        #[derive(Serialize)]
        struct S {
            #[serde(rename = "custom_name")]
            original: String,
        }

        let s = S {
            original: "value".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("custom_name = value"));
        assert!(!ccl.contains("original"));
    }

    #[test]
    fn test_skip_field() {
        #[derive(Serialize)]
        struct S {
            visible: String,
            #[serde(skip)]
            hidden: String,
        }

        let s = S {
            visible: "shown".to_string(),
            hidden: "secret".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("visible = shown"));
        assert!(!ccl.contains("hidden"));
        assert!(!ccl.contains("secret"));
    }

    #[test]
    fn test_skip_serializing_if() {
        #[derive(Serialize)]
        struct S {
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            optional: Option<String>,
        }

        let s = S {
            name: "test".to_string(),
            optional: None,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("name = test"));
        assert!(!ccl.contains("optional"));
    }

    #[test]
    fn test_flatten() {
        #[derive(Serialize)]
        struct Inner {
            inner_field: String,
        }

        #[derive(Serialize)]
        struct Outer {
            outer_field: String,
            #[serde(flatten)]
            inner: Inner,
        }

        let s = Outer {
            outer_field: "outer".to_string(),
            inner: Inner {
                inner_field: "inner".to_string(),
            },
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("outer_field = outer"));
        assert!(ccl.contains("inner_field = inner"));
    }

    // ===========================================
    // Custom Printer Configuration
    // ===========================================

    #[test]
    fn test_custom_printer_config() {
        #[derive(Serialize)]
        struct S {
            name: String,
        }

        let s = S {
            name: "test".to_string(),
        };

        let config = PrinterConfig {
            indent_size: 4, // 4 spaces instead of default 2
            ..Default::default()
        };

        let ccl = to_string_with_config(&s, config).unwrap();
        assert!(ccl.contains("name = test"));
    }

    // ===========================================
    // Round-Trip Tests
    // ===========================================

    #[test]
    fn test_roundtrip_simple() {
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

    #[test]
    fn test_roundtrip_with_option() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            #[serde(default)]
            description: Option<String>,
        }

        let original = Config {
            name: "App".to_string(),
            description: Some("A description".to_string()),
        };

        let ccl = to_string(&original).unwrap();
        let parsed: Config = crate::from_str(&ccl).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_roundtrip_floats() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            ratio: f64,
        }

        let original = Config { ratio: 3.14159 };

        let ccl = to_string(&original).unwrap();
        let parsed: Config = crate::from_str(&ccl).unwrap();

        assert!((original.ratio - parsed.ratio).abs() < 1e-10);
    }

    #[test]
    fn test_issue_69_hashmap_string_string_roundtrip() {
        use std::collections::HashMap;

        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());

        // Serialize to CCL
        let ccl = to_string(&map).unwrap();
        println!("Serialized: {}", ccl);

        // Try to deserialize back
        let result: crate::Result<HashMap<String, String>> = crate::from_str(&ccl);

        match result {
            Ok(deserialized) => {
                assert_eq!(map, deserialized);
                println!("Roundtrip successful!");
            }
            Err(e) => {
                println!("Roundtrip failed: {}", e);
                panic!("HashMap<String, String> roundtrip failed: {}", e);
            }
        }
    }

    #[test]
    fn test_issue_68_hashmap_struct_roundtrip() {
        use std::collections::HashMap;

        #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
        struct ConfigValue {
            name: String,
            value: i32,
        }

        let mut map: HashMap<String, ConfigValue> = HashMap::new();
        map.insert(
            "item1".to_string(),
            ConfigValue {
                name: "first".to_string(),
                value: 1,
            },
        );
        map.insert(
            "item2".to_string(),
            ConfigValue {
                name: "second".to_string(),
                value: 2,
            },
        );

        // Serialize to CCL
        let ccl = to_string(&map).unwrap();
        println!("Serialized: {}", ccl);

        // Try to deserialize back
        let result: crate::Result<HashMap<String, ConfigValue>> = crate::from_str(&ccl);

        match result {
            Ok(deserialized) => {
                assert_eq!(map, deserialized);
                println!("Roundtrip successful!");
            }
            Err(e) => {
                println!("Roundtrip failed: {}", e);
                panic!("HashMap<String, Struct> roundtrip failed: {}", e);
            }
        }
    }

    #[test]
    fn test_issue_67_vec_struct_roundtrip() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Item {
            name: String,
            value: i32,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Container {
            #[serde(default)]
            items: Vec<Item>,
        }

        let original = Container {
            items: vec![
                Item {
                    name: "first".to_string(),
                    value: 1,
                },
                Item {
                    name: "second".to_string(),
                    value: 2,
                },
            ],
        };

        // Serialize to CCL
        let ccl = to_string(&original).unwrap();
        println!("Serialized: {}", ccl);

        // Try to deserialize back
        let result: crate::Result<Container> = crate::from_str(&ccl);

        match result {
            Ok(deserialized) => {
                assert_eq!(original, deserialized);
                println!("Roundtrip successful!");
            }
            Err(e) => {
                println!("Roundtrip failed: {}", e);
                panic!("Vec<Struct> roundtrip failed: {}", e);
            }
        }
    }

    #[test]
    fn test_roundtrip_all_integer_types() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct AllInts {
            a: i8,
            b: i16,
            c: i32,
            d: i64,
            e: u8,
            f: u16,
            g: u32,
            h: u64,
        }

        let original = AllInts {
            a: -1,
            b: -2,
            c: -3,
            d: -4,
            e: 1,
            f: 2,
            g: 3,
            h: 4,
        };

        let ccl = to_string(&original).unwrap();
        let parsed: AllInts = crate::from_str(&ccl).unwrap();

        assert_eq!(original, parsed);
    }

    // ===========================================
    // Edge Cases
    // ===========================================

    #[test]
    fn test_empty_string() {
        #[derive(Serialize)]
        struct S {
            value: String,
        }

        let s = S {
            value: String::new(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value ="));
    }

    #[test]
    fn test_string_with_spaces() {
        #[derive(Serialize)]
        struct S {
            value: String,
        }

        let s = S {
            value: "hello world with spaces".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = hello world with spaces"));
    }

    #[test]
    fn test_unicode_content() {
        #[derive(Serialize)]
        struct S {
            value: String,
        }

        let s = S {
            value: "日本語テスト".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("value = 日本語テスト"));
    }

    #[test]
    fn test_emoji_content() {
        #[derive(Serialize)]
        struct S {
            value: String,
        }

        let s = S {
            value: "🎉🚀✨".to_string(),
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("🎉🚀✨"));
    }

    #[test]
    fn test_zero_values() {
        #[derive(Serialize)]
        struct S {
            int_zero: i32,
            float_zero: f64,
        }

        let s = S {
            int_zero: 0,
            float_zero: 0.0,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("int_zero = 0"));
        assert!(ccl.contains("float_zero = 0"));
    }

    #[test]
    fn test_negative_numbers() {
        #[derive(Serialize)]
        struct S {
            negative_int: i32,
            negative_float: f64,
        }

        let s = S {
            negative_int: -42,
            negative_float: -3.14,
        };
        let ccl = to_string(&s).unwrap();
        assert!(ccl.contains("negative_int = -42"));
        assert!(ccl.contains("negative_float = -3.14"));
    }

    // ===========================================
    // Serializer Internal Tests
    // ===========================================

    #[test]
    fn test_serializer_default() {
        let ser = Serializer::default();
        assert!(ser.current_key.is_none());
        assert_eq!(ser.stack.len(), 1);
    }

    #[test]
    fn test_ser_error_display() {
        let err = SerError::custom("test error message");
        assert_eq!(format!("{}", err), "test error message");
    }

    #[test]
    fn test_ser_error_into_ccl_error() {
        let ser_err = SerError::custom("serialization failed");
        let ccl_err: crate::Error = ser_err.into();
        match ccl_err {
            crate::Error::ValueError(msg) => assert_eq!(msg, "serialization failed"),
            _ => panic!("Expected ValueError"),
        }
    }
}

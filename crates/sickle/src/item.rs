//! The node type of a Sickle document tree.

use crate::error::ExpectedType;
use crate::table::{Array, Table};
use crate::value::Value;
use std::ops::{Index, IndexMut};

/// A node in a CCL document.
///
/// CCL has three shapes of node — a scalar, a nested block, and a list — plus
/// [`Item::None`] for "not present", which lets [`IndexMut`] auto-vivify
/// missing keys the way `toml_edit` does.
///
/// There is deliberately no inline-table or array-of-tables variant: CCL cannot
/// spell either one.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Item {
    /// No value. Rendered as nothing and skipped by iteration helpers.
    #[default]
    None,
    /// A scalar.
    Value(Value),
    /// A nested block.
    Table(Table),
    /// A list.
    Array(Array),
}

impl Item {
    /// Whether this node is absent.
    pub fn is_none(&self) -> bool {
        matches!(self, Item::None)
    }

    /// Whether this node is a scalar.
    pub fn is_value(&self) -> bool {
        matches!(self, Item::Value(_))
    }

    /// Whether this node is a nested block.
    pub fn is_table(&self) -> bool {
        matches!(self, Item::Table(_))
    }

    /// Whether this node is a list.
    pub fn is_array(&self) -> bool {
        matches!(self, Item::Array(_))
    }

    /// The scalar stored here, if any.
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Item::Value(v) => Some(v),
            _ => None,
        }
    }

    /// Mutable access to the scalar stored here, if any.
    pub fn as_value_mut(&mut self) -> Option<&mut Value> {
        match self {
            Item::Value(v) => Some(v),
            _ => None,
        }
    }

    /// The nested block stored here, if any.
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Item::Table(t) => Some(t),
            _ => None,
        }
    }

    /// Mutable access to the nested block stored here, if any.
    pub fn as_table_mut(&mut self) -> Option<&mut Table> {
        match self {
            Item::Table(t) => Some(t),
            _ => None,
        }
    }

    /// The list stored here, if any.
    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Item::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Mutable access to the list stored here, if any.
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match self {
            Item::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Consume the node, returning the scalar it held.
    pub fn into_value(self) -> Result<Value, Self> {
        match self {
            Item::Value(v) => Ok(v),
            other => Err(other),
        }
    }

    /// Consume the node, returning the block it held.
    pub fn into_table(self) -> Result<Table, Self> {
        match self {
            Item::Table(t) => Ok(t),
            other => Err(other),
        }
    }

    /// Consume the node, returning the list it held.
    pub fn into_array(self) -> Result<Array, Self> {
        match self {
            Item::Array(a) => Ok(a),
            other => Err(other),
        }
    }

    /// The scalar text stored here, if this node is a scalar.
    pub fn as_str(&self) -> Option<&str> {
        self.as_value().map(|v| v.as_str())
    }

    /// The scalar read as an `i64`.
    pub fn as_integer(&self) -> Option<i64> {
        self.as_value().and_then(|v| v.as_integer())
    }

    /// The scalar read as an `f64`.
    pub fn as_float(&self) -> Option<f64> {
        self.as_value().and_then(|v| v.as_float())
    }

    /// The scalar read as a strict boolean (`true`/`false`).
    pub fn as_bool(&self) -> Option<bool> {
        self.as_value().and_then(|v| v.as_bool())
    }

    /// Replace this node with `item` when it is currently absent, then return a
    /// mutable reference to it.
    pub fn or_insert(&mut self, item: impl Into<Item>) -> &mut Item {
        if self.is_none() {
            *self = item.into();
        }
        self
    }

    /// The node kind, for diagnostics.
    pub fn type_name(&self) -> ExpectedType {
        match self {
            Item::None => ExpectedType::None,
            Item::Value(_) => ExpectedType::String,
            Item::Table(_) => ExpectedType::Table,
            Item::Array(_) => ExpectedType::List,
        }
    }

    /// Canonicalize this node and its descendants.
    pub fn fmt(&mut self) {
        match self {
            Item::None => {}
            Item::Value(v) => v.fmt(),
            Item::Table(t) => t.fmt(),
            Item::Array(a) => a.fmt(),
        }
    }
}

/// Build a scalar [`Item`].
///
/// ```
/// use sickle::{value, DocumentMut};
///
/// let mut doc = DocumentMut::new();
/// doc.as_table_mut().insert("port", value(8080));
/// assert_eq!(doc.to_string(), "port = 8080");
/// ```
pub fn value(value: impl Into<Value>) -> Item {
    Item::Value(value.into())
}

/// Build an empty nested block [`Item`].
pub fn table() -> Item {
    Item::Table(Table::new())
}

/// Build an empty list [`Item`].
pub fn array() -> Item {
    Item::Array(Array::new())
}

impl From<Value> for Item {
    fn from(v: Value) -> Self {
        Item::Value(v)
    }
}

impl From<Table> for Item {
    fn from(t: Table) -> Self {
        Item::Table(t)
    }
}

impl From<Array> for Item {
    fn from(a: Array) -> Self {
        Item::Array(a)
    }
}

impl From<&str> for Item {
    fn from(v: &str) -> Self {
        Item::Value(Value::new(v))
    }
}

impl From<String> for Item {
    fn from(v: String) -> Self {
        Item::Value(Value::new(v))
    }
}

impl From<&String> for Item {
    fn from(v: &String) -> Self {
        Item::Value(Value::new(v.clone()))
    }
}

impl From<bool> for Item {
    fn from(v: bool) -> Self {
        Item::Value(Value::from(v))
    }
}

macro_rules! item_from_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Item {
                fn from(v: $ty) -> Self {
                    Item::Value(Value::from(v))
                }
            }
        )*
    };
}

item_from_number!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl<T: Into<Item>> From<Vec<T>> for Item {
    fn from(values: Vec<T>) -> Self {
        Item::Array(values.into_iter().collect())
    }
}

impl Index<&str> for Item {
    type Output = Item;

    /// Panics unless this node is a table containing `key`.
    fn index(&self, key: &str) -> &Item {
        self.as_table()
            .unwrap_or_else(|| panic!("cannot index `{key}`: node is not a table"))
            .index(key)
    }
}

impl IndexMut<&str> for Item {
    /// Turns [`Item::None`] into a table and auto-vivifies `key`.
    ///
    /// # Panics
    ///
    /// Panics when this node is a scalar or a list.
    fn index_mut(&mut self, key: &str) -> &mut Item {
        if self.is_none() {
            *self = table();
        }
        self.as_table_mut()
            .unwrap_or_else(|| panic!("cannot index `{key}`: node is not a table"))
            .index_mut(key)
    }
}

impl Index<usize> for Item {
    type Output = Item;

    /// Panics unless this node is a list with an element at `index`.
    fn index(&self, index: usize) -> &Item {
        self.as_array()
            .unwrap_or_else(|| panic!("cannot index {index}: node is not a list"))
            .index(index)
    }
}

impl IndexMut<usize> for Item {
    /// Panics unless this node is a list with an element at `index`.
    fn index_mut(&mut self, index: usize) -> &mut Item {
        self.as_array_mut()
            .unwrap_or_else(|| panic!("cannot index {index}: node is not a list"))
            .index_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_produce_expected_shapes() {
        assert!(value("x").is_value());
        assert!(table().is_table());
        assert!(array().is_array());
        assert!(Item::None.is_none());
    }

    #[test]
    fn conversions_are_checked() {
        let item = value("42");
        assert_eq!(item.as_integer(), Some(42));
        assert_eq!(item.as_table(), None);
        assert_eq!(item.type_name(), ExpectedType::String);
    }

    #[test]
    fn vec_converts_to_array() {
        let item: Item = vec!["a", "b"].into();
        let array = item.as_array().unwrap();
        assert_eq!(array.as_strings(), Some(vec!["a", "b"]));
    }

    #[test]
    fn or_insert_only_fills_none() {
        let mut item = Item::None;
        item.or_insert(value("filled"));
        assert_eq!(item.as_str(), Some("filled"));
        item.or_insert(value("ignored"));
        assert_eq!(item.as_str(), Some("filled"));
    }

    #[test]
    fn index_mut_builds_nested_tables() {
        let mut item = Item::None;
        item["server"]["host"] = value("localhost");
        assert_eq!(item["server"]["host"].as_str(), Some("localhost"));
    }
}

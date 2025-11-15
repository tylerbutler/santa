//! Lower-level CCL parsing utilities

use serde_json::Value;

/// Represents a parsed CCL value
#[derive(Debug, Clone, PartialEq)]
pub enum CclValue {
    /// A simple string value
    String(String),
    /// An array of values
    Array(Vec<String>),
    /// An object with key-value pairs
    Object(Vec<(String, CclValue)>),
}

impl From<CclValue> for Value {
    fn from(ccl: CclValue) -> Self {
        match ccl {
            CclValue::String(s) => Value::String(s),
            CclValue::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::String).collect())
            }
            CclValue::Object(pairs) => {
                let mut map = serde_json::Map::new();
                for (k, v) in pairs {
                    map.insert(k, v.into());
                }
                Value::Object(map)
            }
        }
    }
}

/// Parse a CCL document into a CclValue
///
/// This is a lower-level API that returns the parsed structure
/// without going through serde_ccl at all.
pub fn parse_ccl(_content: &str) -> Result<CclValue, String> {
    // For now, this is a placeholder
    // A full implementation would parse CCL syntax directly
    Err("Direct CCL parsing not yet implemented".to_string())
}

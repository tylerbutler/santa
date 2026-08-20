use serde_json::Value;
use sickle::{DocumentMut, Item};

/// Convert a CCL document into a JSON value.
///
/// Scalars become strings, nested blocks become objects, and lists — whether
/// spelled as a bare-list block or as a repeated key — become arrays.
pub(crate) fn document_to_value(document: &DocumentMut) -> Value {
    table_to_value(document.as_table())
}

fn table_to_value(table: &sickle::Table) -> Value {
    let mut map = serde_json::Map::new();
    for key in table.unique_keys() {
        let item = table.get_composed(key).expect("key came from the table");
        map.insert(key.to_string(), item_to_value(&item));
    }
    Value::Object(map)
}

pub(crate) fn item_to_value(item: &Item) -> Value {
    match item {
        Item::Value(scalar) => Value::String(scalar.as_str().to_string()),
        Item::Array(array) => Value::Array(array.iter().map(item_to_value).collect()),
        Item::Table(table) => table_to_value(table),
        Item::None => Value::String(String::new()),
    }
}

pub(crate) fn value_to_ccl_string(value: &Value) -> String {
    value_to_ccl_lines(value, 0).join("\n")
}

fn value_to_ccl_lines(value: &Value, indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            let mut lines = Vec::new();
            for (key, val) in map {
                match val {
                    Value::Object(_) => {
                        lines.push(format!("{}{} =", prefix, key));
                        lines.extend(value_to_ccl_lines(val, indent + 2));
                    }
                    Value::Array(arr) => {
                        lines.push(format!("{}{} =", prefix, key));
                        for item in arr {
                            match item {
                                Value::Object(_) | Value::Array(_) => {
                                    lines.extend(value_to_ccl_lines(item, indent + 2));
                                }
                                _ => {
                                    let s = scalar_to_string(item);
                                    lines.push(format!("{}  = {}", prefix, s));
                                }
                            }
                        }
                    }
                    _ => {
                        let s = scalar_to_string(val);
                        lines.push(format!("{}{} = {}", prefix, key, s));
                    }
                }
            }
            lines
        }
        Value::Array(arr) => {
            let mut lines = Vec::new();
            for item in arr {
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        lines.extend(value_to_ccl_lines(item, indent));
                    }
                    _ => {
                        let s = scalar_to_string(item);
                        lines.push(format!("{}= {}", prefix, s));
                    }
                }
            }
            lines
        }
        _ => {
            vec![format!("{}{}", prefix, scalar_to_string(value))]
        }
    }
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => {
            eprintln!("warning: null value converted to empty string");
            String::new()
        }
        _ => unreachable!("scalar_to_string called with non-scalar"),
    }
}

pub(crate) fn has_comments(ccl_text: &str) -> bool {
    ccl_text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("/=") || trimmed.starts_with("/ =")
    })
}

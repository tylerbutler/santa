use serde_json::Value;
use sickle::CclObject;

pub fn ccl_to_value(obj: &CclObject) -> Value {
    if obj.is_empty() {
        return Value::String(String::new());
    }

    let keys: Vec<&String> = obj.keys().collect();

    // Check for string value first: a single key with one empty child
    // This handles both regular strings like {"Alice": [{}]} and empty
    // strings like {"": [{}]}
    if keys.len() == 1 {
        if let Ok(values) = obj.get_all(keys[0]) {
            if values.len() == 1 && values[0].is_empty() {
                return Value::String(keys[0].clone());
            }
        }
    }

    // Check for bare list: single empty key with multiple values
    if keys.len() == 1 && keys[0].is_empty() {
        if let Ok(items) = obj.get_all("") {
            let arr: Vec<Value> = items.iter().map(ccl_to_value).collect();
            return Value::Array(arr);
        }
    }

    let mut map = serde_json::Map::new();
    for key in obj.keys() {
        if let Ok(values) = obj.get_all(key) {
            if values.len() == 1 {
                map.insert(key.clone(), ccl_to_value(&values[0]));
            } else {
                let arr: Vec<Value> = values.iter().map(ccl_to_value).collect();
                map.insert(key.clone(), Value::Array(arr));
            }
        }
    }
    Value::Object(map)
}

pub fn value_to_ccl_string(value: &Value) -> String {
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

pub fn has_comments(ccl_text: &str) -> bool {
    ccl_text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("/=") || trimmed.starts_with("/ =")
    })
}

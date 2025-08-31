/// YAML to HOCON conversion utilities
/// 
/// This module handles the conversion of YAML configuration files to HOCON format.
/// It focuses on preserving semantics while improving readability and taking advantage
/// of HOCON's more flexible syntax.

use anyhow::{Context, Result};
use serde_json::Value;

/// Convert YAML content to HOCON format
/// 
/// This function parses YAML into a JSON Value intermediate representation,
/// then formats it as HOCON with improved syntax and readability.
pub fn convert_yaml_to_hocon(yaml_content: &str) -> Result<String> {
    // Parse YAML to JSON Value (universal intermediate format)
    let value: Value = serde_yaml::from_str(yaml_content)
        .context("Failed to parse YAML content")?;
    
    // Convert JSON Value to HOCON string
    format_value_as_hocon(&value, 0)
}

/// Format a JSON Value as HOCON with proper indentation and syntax
fn format_value_as_hocon(value: &Value, indent_level: usize) -> Result<String> {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                return Ok("{}".to_string());
            }
            
            let mut result = String::new();
            let _indent = "  ".repeat(indent_level);
            let child_indent = "  ".repeat(indent_level + 1);
            
            let mut items = map.iter().collect::<Vec<_>>();
            items.sort_by_key(|(k, _)| *k); // Sort keys for consistent output
            
            for (i, (key, val)) in items.iter().enumerate() {
                if i > 0 {
                    result.push('\n');
                }
                
                result.push_str(&child_indent);
                
                // Format key (quote only if necessary)
                let formatted_key = if needs_quoting(key) {
                    format!("\"{}\"", key)
                } else {
                    key.to_string()
                };
                
                result.push_str(&formatted_key);
                
                match val {
                    Value::Object(_) => {
                        result.push_str(" {\n");
                        result.push_str(&format_value_as_hocon(val, indent_level + 2)?);
                        result.push('\n');
                        result.push_str(&child_indent);
                        result.push('}');
                    }
                    Value::Array(_) => {
                        result.push_str(" = ");
                        result.push_str(&format_value_as_hocon(val, indent_level + 1)?);
                    }
                    _ => {
                        result.push_str(" = ");
                        result.push_str(&format_value_as_hocon(val, indent_level + 1)?);
                    }
                }
            }
            
            Ok(result)
        }
        
        Value::Array(arr) => {
            if arr.is_empty() {
                return Ok("[]".to_string());
            }
            
            // Check if all items are simple values (strings, numbers, booleans)
            let all_simple = arr.iter().all(|v| matches!(v, 
                Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
            ));
            
            if all_simple && arr.len() <= 5 {
                // Compact format for short, simple arrays
                let items: Result<Vec<String>, _> = arr.iter()
                    .map(|v| format_value_as_hocon(v, 0))
                    .collect();
                Ok(format!("[{}]", items?.join(", ")))
            } else {
                // Multi-line format for complex or long arrays
                let mut result = String::from("[\n");
                let child_indent = "  ".repeat(indent_level + 1);
                
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        result.push('\n');
                    }
                    result.push_str(&child_indent);
                    result.push_str(&format_value_as_hocon(item, indent_level + 1)?);
                }
                
                result.push('\n');
                result.push_str(&"  ".repeat(indent_level));
                result.push(']');
                Ok(result)
            }
        }
        
        Value::String(s) => {
            // Quote only if necessary for HOCON
            if needs_quoting(s) {
                Ok(format!("\"{}\"", escape_string(s)))
            } else {
                Ok(s.clone())
            }
        }
        
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok("null".to_string()),
    }
}

/// Check if a string needs to be quoted in HOCON
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    
    // Check for HOCON keywords that need quoting
    match s {
        "true" | "false" | "null" | "include" | "substitution" => return true,
        _ => {}
    }
    
    // Check for special characters that require quoting
    s.chars().any(|c| match c {
        ' ' | '\t' | '\n' | '\r' | '"' | '\'' | '\\' | 
        '{' | '}' | '[' | ']' | '=' | ':' | ',' | 
        '#' | '!' | '@' | '$' | '%' | '^' | '&' | 
        '*' | '(' | ')' | '+' | '|' | '?' | '<' | '>' => true,
        _ => false,
    }) || s.starts_with('-') || s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Escape special characters in strings
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_simple_object_conversion() -> Result<()> {
        let yaml = r#"
sources:
  - npm
  - cargo
packages:
  - git
  - rust
"#;
        
        let hocon = convert_yaml_to_hocon(yaml)?;
        println!("Converted HOCON:\n{}", hocon);
        
        assert!(hocon.contains("sources = [npm, cargo]"));
        assert!(hocon.contains("packages = [git, rust]"));
        Ok(())
    }
    
    #[test]
    fn test_nested_object_conversion() -> Result<()> {
        let yaml = r#"
database:
  host: localhost
  port: 5432
  credentials:
    username: user
    password: pass
"#;
        
        let hocon = convert_yaml_to_hocon(yaml)?;
        println!("Converted HOCON:\n{}", hocon);
        
        assert!(hocon.contains("database {"));
        assert!(hocon.contains("host = localhost"));
        assert!(hocon.contains("credentials {"));
        Ok(())
    }
    
    #[test]
    fn test_needs_quoting() {
        // Should not need quoting
        assert!(!needs_quoting("simple"));
        assert!(!needs_quoting("camelCase"));
        assert!(!needs_quoting("under_score"));
        assert!(!needs_quoting("with-dash"));
        
        // Should need quoting
        assert!(needs_quoting("with space"));
        assert!(needs_quoting("true"));
        assert!(needs_quoting("false"));
        assert!(needs_quoting("null"));
        assert!(needs_quoting("123"));
        assert!(needs_quoting("with:colon"));
        assert!(needs_quoting("with\"quote"));
    }
    
    #[test]
    fn test_array_formatting() -> Result<()> {
        let simple_array = json!(["one", "two", "three"]);
        let formatted = format_value_as_hocon(&simple_array, 0)?;
        assert_eq!(formatted, "[one, two, three]");
        
        let complex_array = json!([
            {"name": "item1", "value": 1},
            {"name": "item2", "value": 2}
        ]);
        let formatted = format_value_as_hocon(&complex_array, 0)?;
        assert!(formatted.contains("[\n"));  // Multi-line format
        assert!(formatted.contains("name = item1"));
        Ok(())
    }
    
    #[test] 
    fn test_escape_string() {
        assert_eq!(escape_string("simple"), "simple");
        assert_eq!(escape_string("with\"quote"), "with\\\"quote");
        assert_eq!(escape_string("with\\slash"), "with\\\\slash");
        assert_eq!(escape_string("with\nnewline"), "with\\nnewline");
    }
    
    #[test]
    fn test_real_santa_config() -> Result<()> {
        let yaml = r#"
sources:
  - brew
  - scoop
  - npm
  - cargo
packages:
  - git
  - node
  - rust
  - rg
  - bat
  - fzf
custom_sources:
  - name: "custom-brew"
    emoji: "🍺"
    install: "brew install {package}"
    check: "brew list"
"#;
        
        let hocon = convert_yaml_to_hocon(yaml)?;
        println!("Real config conversion:\n{}", hocon);
        
        // Verify key aspects are preserved
        assert!(hocon.contains("sources = [brew, scoop, npm, cargo]"));
        assert!(hocon.contains("packages = ["));
        assert!(hocon.contains("custom_sources"));
        assert!(hocon.contains("name = \"custom-brew\""));
        
        Ok(())
    }
}
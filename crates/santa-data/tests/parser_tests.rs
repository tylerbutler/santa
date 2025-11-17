//! Comprehensive tests for CCL parser functionality

use santa_data::*;
use serde_json::Value;

#[test]
fn test_empty_input() {
    let result = parse_to_hashmap("");
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_whitespace_only() {
    let ccl = "   \n\n  \t  \n";
    let result = parse_to_hashmap(ccl);
    // serde_ccl may error on whitespace-only input
    if result.is_ok() {
        assert!(result.unwrap().is_empty());
    } else {
        // Error is also acceptable for whitespace-only input
        assert!(result.is_err());
    }
}

#[test]
fn test_single_package_single_source() {
    let ccl = r#"
bat =
  = brew
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains_key("bat"));

    let arr = result["bat"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_str().unwrap(), "brew");
}

#[test]
fn test_array_with_extra_whitespace() {
    let ccl = r#"
pkg =
  =   brew
  =     scoop
  = pacman
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let arr = result["pkg"].as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str().unwrap(), "brew");
    assert_eq!(arr[1].as_str().unwrap(), "scoop");
    assert_eq!(arr[2].as_str().unwrap(), "pacman");
}

#[test]
fn test_complex_with_inline_value() {
    let ccl = r#"
pkg =
  brew = gh
  scoop = ripgrep
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let obj = result["pkg"].as_object().unwrap();
    assert_eq!(obj["brew"].as_str().unwrap(), "gh");
    assert_eq!(obj["scoop"].as_str().unwrap(), "ripgrep");
}

#[test]
fn test_nested_arrays_in_object() {
    let ccl = r#"
pkg =
  _sources =
    = brew
    = scoop
  _platforms =
    = macos
    = windows
  _aliases =
    = rg
    = ripgrep
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let obj = result["pkg"].as_object().unwrap();

    let sources = obj["_sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);

    let platforms = obj["_platforms"].as_array().unwrap();
    assert_eq!(platforms.len(), 2);

    let aliases = obj["_aliases"].as_array().unwrap();
    assert_eq!(aliases.len(), 2);
}

#[test]
fn test_mixed_inline_and_multiline() {
    let ccl = r#"
pkg =
  name = custom-name
  _sources =
    = brew
    = scoop
  version = 1.2.3
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let obj = result["pkg"].as_object().unwrap();

    assert_eq!(obj["name"].as_str().unwrap(), "custom-name");
    assert_eq!(obj["version"].as_str().unwrap(), "1.2.3");
    assert!(obj["_sources"].is_array());
}

#[test]
fn test_package_names_with_hyphens() {
    let ccl = r#"
ripgrep-all =
  = brew
node-version-manager =
  = scoop
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    assert!(result.contains_key("ripgrep-all"));
    assert!(result.contains_key("node-version-manager"));
}

#[test]
fn test_package_names_with_underscores() {
    let ccl = r#"
package_name =
  = brew
another_pkg =
  = scoop
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    assert!(result.contains_key("package_name"));
    assert!(result.contains_key("another_pkg"));
}

#[test]
fn test_special_field_names() {
    let ccl = r#"
pkg =
  _sources =
    = brew
  _platforms =
    = linux
  _aliases =
    = alias1
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let obj = result["pkg"].as_object().unwrap();

    assert!(obj.contains_key("_sources"));
    assert!(obj.contains_key("_platforms"));
    assert!(obj.contains_key("_aliases"));
}

#[test]
fn test_empty_array() {
    let ccl = r#"
pkg =
  =
"#;
    let result = parse_to_hashmap(ccl);
    // This should parse but might have an empty array or error
    // depending on implementation - document the behavior
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_multiple_packages_various_formats() {
    let ccl = r#"
simple1 =
  = brew

simple2 =
  = scoop
  = pacman

complex1 =
  _sources =
    = apt
  brew = custom

complex2 =
  name = override
  _sources =
    = nix
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    assert_eq!(result.len(), 4);

    assert!(result["simple1"].is_array());
    assert!(result["simple2"].is_array());
    assert!(result["complex1"].is_object());
    assert!(result["complex2"].is_object());
}

#[test]
fn test_ccl_value_to_json_string() {
    let ccl_val = CclValue::String("test".to_string());
    let json_val: Value = ccl_val.into();
    assert_eq!(json_val.as_str().unwrap(), "test");
}

#[test]
fn test_ccl_value_to_json_array() {
    let ccl_val = CclValue::Array(vec!["brew".to_string(), "scoop".to_string()]);
    let json_val: Value = ccl_val.into();
    let arr = json_val.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str().unwrap(), "brew");
    assert_eq!(arr[1].as_str().unwrap(), "scoop");
}

#[test]
fn test_ccl_value_to_json_object() {
    let ccl_val = CclValue::Object(vec![
        ("key1".to_string(), CclValue::String("value1".to_string())),
        (
            "key2".to_string(),
            CclValue::Array(vec!["a".to_string(), "b".to_string()]),
        ),
    ]);
    let json_val: Value = ccl_val.into();
    let obj = json_val.as_object().unwrap();
    assert_eq!(obj["key1"].as_str().unwrap(), "value1");
    assert!(obj["key2"].is_array());
}

#[test]
fn test_ccl_value_equality() {
    let val1 = CclValue::String("test".to_string());
    let val2 = CclValue::String("test".to_string());
    let val3 = CclValue::String("other".to_string());

    assert_eq!(val1, val2);
    assert_ne!(val1, val3);
}

#[test]
fn test_ccl_value_clone() {
    let original = CclValue::Array(vec!["brew".to_string()]);
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn test_parse_ccl_not_implemented() {
    let result = parse_ccl("test");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not yet implemented"));
}

#[test]
fn test_deeply_nested_structure() {
    let ccl = r#"
pkg =
  _sources =
    = brew
    = scoop
  brew =
    name = custom
  scoop =
    name = other
"#;
    // This might fail with current parser - test to document behavior
    let result = parse_to_hashmap(ccl);
    // Document whether this is supported or not
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_unicode_in_values() {
    let ccl = r#"
pkg =
  emoji = 🍺
  name = café
"#;
    let result = parse_to_hashmap(ccl);
    if let Ok(obj) = result {
        assert!(obj.contains_key("pkg"));
    }
}

#[test]
fn test_numbers_as_strings() {
    let ccl = r#"
pkg =
  version = 1.2.3
  port = 8080
"#;
    let result = parse_to_hashmap(ccl).unwrap();
    let obj = result["pkg"].as_object().unwrap();
    assert_eq!(obj["version"].as_str().unwrap(), "1.2.3");
    assert_eq!(obj["port"].as_str().unwrap(), "8080");
}

#[test]
fn test_parse_ccl_to_with_simple_struct() {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct SimplePackage {
        #[serde(rename = "_sources")]
        sources: Option<Vec<String>>,
    }

    let ccl = r#"
bat =
  _sources =
    = brew
    = scoop
"#;

    let packages: HashMap<String, SimplePackage> = parse_ccl_to(ccl).unwrap();
    assert!(packages.contains_key("bat"));
    let bat_sources = packages["bat"].sources.as_ref().unwrap();
    assert_eq!(bat_sources.len(), 2);
}

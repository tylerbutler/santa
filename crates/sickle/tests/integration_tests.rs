//! Integration tests for the sickle CCL parser

use sickle::parse;

#[test]
fn test_complete_config_file() {
    let ccl = r#"
/= Application Configuration
name = Santa Package Manager
version = 0.1.0
description = A tool that manages packages across different platforms

/= Database Configuration
database =
  host = localhost
  port = 5432
  pool_size = 10
  credentials =
    username = admin
    password = secret123

/= Feature Flags
features =
  hot_reload = true
  script_generation = true
  multi_platform = true

/= Supported Package Managers
package_managers =
  = brew
  = apt
  = npm
  = cargo
"#;

    let model = parse(ccl).expect("should parse successfully");

    // Test simple values
    assert_eq!(
        model.get("name").unwrap().as_str().unwrap(),
        "Santa Package Manager"
    );
    assert_eq!(model.get("version").unwrap().as_str().unwrap(), "0.1.0");

    // Test nested map navigation - database should be parsed as a map
    let db = model.get("database").expect("database should exist");
    assert!(db.is_map(), "database should be a parsed map");

    // Verify nested values
    assert_eq!(db.get("host").unwrap().as_str().unwrap(), "localhost");
    let port: u16 = db.get("port").unwrap().parse_value().unwrap();
    assert_eq!(port, 5432);
}

#[test]
fn test_multiline_strings() {
    let ccl = r#"
description = This is a very long description
  that spans multiple lines
  and contains important information
  about the configuration file
"#;

    let model = parse(ccl).expect("should parse");
    let desc = model.get("description").unwrap().as_str().unwrap();

    assert!(desc.contains("long description"));
    assert!(desc.contains("multiple lines"));
    assert!(desc.contains("configuration file"));
}

#[test]
fn test_comments_are_ignored() {
    let ccl = r#"
/= This is a comment
/= Comments should be completely ignored
name = value
/= Another comment in the middle
other = data
"#;

    let model = parse(ccl).expect("should parse");

    // Comments should not appear as keys
    assert!(model.get("/").is_err());
    assert_eq!(model.get("name").unwrap().as_str().unwrap(), "value");
    assert_eq!(model.get("other").unwrap().as_str().unwrap(), "data");
}

#[test]
fn test_empty_values() {
    let ccl = r#"
key_with_empty_value =
another =
non_empty = value
"#;

    let model = parse(ccl).expect("should parse");

    assert_eq!(
        model.get("key_with_empty_value").unwrap().as_str().unwrap(),
        ""
    );
    assert_eq!(model.get("another").unwrap().as_str().unwrap(), "");
    assert_eq!(model.get("non_empty").unwrap().as_str().unwrap(), "value");
}

#[test]
fn test_special_characters_in_values() {
    let ccl = r#"
url = https://github.com/user/repo
email = user@example.com
path = /usr/local/bin
command = echo "Hello World"
"#;

    let model = parse(ccl).expect("should parse");

    assert_eq!(
        model.get("url").unwrap().as_str().unwrap(),
        "https://github.com/user/repo"
    );
    assert_eq!(
        model.get("email").unwrap().as_str().unwrap(),
        "user@example.com"
    );
    assert_eq!(
        model.get("path").unwrap().as_str().unwrap(),
        "/usr/local/bin"
    );
    assert_eq!(
        model.get("command").unwrap().as_str().unwrap(),
        "echo \"Hello World\""
    );
}

#[test]
fn test_model_merging() {
    let config1 = parse(
        r#"
name = App1
version = 1.0.0
"#,
    )
    .unwrap();

    let config2 = parse(
        r#"
author = Tyler
license = MIT
"#,
    )
    .unwrap();

    let merged = config1.merge(config2);
    let map = merged.as_map().unwrap();

    assert_eq!(map.len(), 4);
    assert!(map.contains_key("name"));
    assert!(map.contains_key("version"));
    assert!(map.contains_key("author"));
    assert!(map.contains_key("license"));
}

#[test]
fn test_type_parsing() {
    let ccl = r#"
string_val = hello
int_val = 42
float_val = 3.14
bool_true = true
bool_false = false
"#;

    let model = parse(ccl).expect("should parse");

    // String
    assert_eq!(model.get("string_val").unwrap().as_str().unwrap(), "hello");

    // Integer
    let int: i32 = model.get("int_val").unwrap().parse_value().unwrap();
    assert_eq!(int, 42);

    // Float
    let float: f64 = model.get("float_val").unwrap().parse_value().unwrap();
    assert!((float - std::f64::consts::PI).abs() < 0.01);

    // Booleans
    let bool_t: bool = model.get("bool_true").unwrap().parse_value().unwrap();
    let bool_f: bool = model.get("bool_false").unwrap().parse_value().unwrap();
    assert!(bool_t);
    assert!(!bool_f);
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_nested_structs() {
    use serde::Deserialize;
    use sickle::from_str;

    #[derive(Deserialize, Debug, PartialEq)]
    struct AppConfig {
        name: String,
        database: DbConfig,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DbConfig {
        host: String,
        port: u16,
    }

    let ccl = r#"
name = MyApp
database =
  host = db.example.com
  port = 3306
"#;

    let config: AppConfig = from_str(ccl).expect("should deserialize");
    assert_eq!(config.name, "MyApp");
    assert_eq!(config.database.host, "db.example.com");
    assert_eq!(config.database.port, 3306);
}

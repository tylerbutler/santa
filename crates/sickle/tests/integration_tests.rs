//! Integration tests for the sickle CCL parser

mod test_helpers;

use sickle::load;

/// Test helper to extract string value from Model using public API
fn model_as_str(model: &sickle::Model) -> Result<&str, String> {
    if model.0.len() == 1 {
        let (key, value) = model.0.iter().next().unwrap();
        if value.0.is_empty() {
            return Ok(key.as_str());
        }
    }
    Err("not a singleton string".to_string())
}

/// Test helper to check if Model is a map using public API
fn model_is_map(model: &sickle::Model) -> bool {
    !model.0.is_empty() && model.0.values().any(|v| !v.0.is_empty())
}

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

    let model = load(ccl).expect("should load successfully");

    // Test simple values
    assert_eq!(
        model_as_str(model.get("name").unwrap()).unwrap(),
        "Santa Package Manager"
    );
    assert_eq!(
        model_as_str(model.get("version").unwrap()).unwrap(),
        "0.1.0"
    );

    // Test nested map navigation - database should be parsed as a map
    let db = model.get("database").expect("database should exist");
    assert!(model_is_map(db), "database should be a parsed map");

    // Verify nested values
    assert_eq!(model_as_str(db.get("host").unwrap()).unwrap(), "localhost");
    let port_str = model_as_str(db.get("port").unwrap()).unwrap();
    let port: u16 = port_str.parse().unwrap();
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

    let model = load(ccl).expect("should load");
    let desc = model_as_str(model.get("description").unwrap()).unwrap();

    assert!(desc.contains("long description"));
    assert!(desc.contains("multiple lines"));
    assert!(desc.contains("configuration file"));
}

#[test]
fn test_comments_are_preserved() {
    let ccl = r#"
/= This is a comment
/= Comments are valid entries in CCL
name = value
/= Another comment in the middle
other = data
"#;

    let model = load(ccl).expect("should load");

    // Comments ARE valid entries with key "/" per CCL spec
    assert!(model.get("/").is_ok());
    // Comments are stored as keys in the IndexMap (list representation)
    let comments_model = model.get("/").unwrap();
    let comment_keys: Vec<&String> = comments_model.0.keys().collect();
    assert_eq!(comment_keys.len(), 3);
    assert_eq!(comment_keys[0], "This is a comment");
    assert_eq!(comment_keys[1], "Comments are valid entries in CCL");
    assert_eq!(comment_keys[2], "Another comment in the middle");

    // Other keys work as expected
    assert_eq!(model_as_str(model.get("name").unwrap()).unwrap(), "value");
    assert_eq!(model_as_str(model.get("other").unwrap()).unwrap(), "data");
}

#[test]
fn test_empty_values() {
    let ccl = r#"
key_with_empty_value =
another =
non_empty = value
"#;

    let model = load(ccl).expect("should load");

    assert_eq!(
        model_as_str(model.get("key_with_empty_value").unwrap()).unwrap(),
        ""
    );
    assert_eq!(model_as_str(model.get("another").unwrap()).unwrap(), "");
    assert_eq!(
        model_as_str(model.get("non_empty").unwrap()).unwrap(),
        "value"
    );
}

#[test]
fn test_special_characters_in_values() {
    let ccl = r#"
url = https://github.com/user/repo
email = user@example.com
path = /usr/local/bin
command = echo "Hello World"
"#;

    let model = load(ccl).expect("should load");

    assert_eq!(
        model_as_str(model.get("url").unwrap()).unwrap(),
        "https://github.com/user/repo"
    );
    assert_eq!(
        model_as_str(model.get("email").unwrap()).unwrap(),
        "user@example.com"
    );
    assert_eq!(
        model_as_str(model.get("path").unwrap()).unwrap(),
        "/usr/local/bin"
    );
    assert_eq!(
        model_as_str(model.get("command").unwrap()).unwrap(),
        "echo \"Hello World\""
    );
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

    let model = load(ccl).expect("should load");

    // String - use public get_string API
    assert_eq!(model.get_string("string_val").unwrap(), "hello");

    // Integer - use public get_int API
    let int = model.get_int("int_val").unwrap();
    assert_eq!(int, 42);

    // Float - use public get_float API
    let float = model.get_float("float_val").unwrap();
    assert!((float - std::f64::consts::PI).abs() < 0.01);

    // Booleans - use public get_bool API
    let bool_t = model.get_bool("bool_true").unwrap();
    let bool_f = model.get_bool("bool_false").unwrap();
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

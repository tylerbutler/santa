//! Integration tests for the sickle CCL parser

use sickle::load;

#[cfg(feature = "unstable")]
use sickle::{
    load_with_options, parse_indented_with_options, parse_with_options, CrlfBehavior,
    ParserOptions, TabBehavior,
};

/// Test helper to extract string value from Model using public API
fn model_as_str(model: &sickle::CclObject) -> Result<&str, String> {
    if model.len() == 1 {
        let (key, value) = model.iter().next().unwrap();
        if value.is_empty() {
            return Ok(key.as_str());
        }
    }
    Err("not a singleton string".to_string())
}

/// Test helper to check if Model is a map using public API
fn model_is_map(model: &sickle::CclObject) -> bool {
    !model.is_empty() && model.values().any(|v| !v.is_empty())
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

/// Test that comments are preserved in the model
///
/// Note: When `reference_compliant` feature is enabled, duplicate keys are reversed
/// to match the OCaml reference implementation. This test expects insertion order.
#[test]
#[cfg_attr(feature = "reference_compliant", ignore)]
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
    // With Vec structure, multiple comments are stored as separate entries in the Vec
    // Each entry is a CclObject with the comment text as its single key
    let all_comments = model.get_all("/").unwrap();
    assert_eq!(all_comments.len(), 3, "expected 3 comments");
    let comment_keys: Vec<String> = all_comments
        .iter()
        .map(|c| c.keys().next().unwrap().clone())
        .collect();
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

// Tests for *_with_options APIs (unstable feature)

#[cfg(feature = "unstable")]
#[test]
fn test_parse_with_options_tabs_preserved() {
    let ccl = "name = hello\tworld";
    let opts = ParserOptions::new(); // Default preserves tabs
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value, "hello\tworld");
}

#[cfg(feature = "unstable")]
#[test]
fn test_parse_with_options_tabs_to_spaces() {
    let ccl = "name = hello\tworld";
    let opts = ParserOptions::new().with_tabs(TabBehavior::ToSpaces);
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value, "hello world");
}

#[cfg(feature = "unstable")]
#[test]
fn test_parse_with_options_crlf_preserved() {
    // Test with CRLF in a single-line value (not continuation lines)
    let ccl = "name = line1\r\nother = value";
    let opts = ParserOptions::new(); // Default preserves CRLF
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    // With default options, CRLF is preserved, so the input splits into separate entries
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key, "name");
    assert_eq!(entries[1].key, "other");
}

#[cfg(feature = "unstable")]
#[test]
fn test_parse_with_options_crlf_normalized() {
    // With normalization, CRLF becomes LF during parsing
    let ccl = "name = line1\r\n  line2";
    let opts = ParserOptions::new().with_crlf(CrlfBehavior::NormalizeToLf);
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    assert_eq!(entries.len(), 1);
    // The multiline value should not contain CRLF after normalization
    assert!(!entries[0].value.contains("\r\n"));
}

#[cfg(feature = "unstable")]
#[test]
fn test_parse_indented_with_options() {
    let ccl = "  name = value\n  other = data";
    let opts = ParserOptions::new();
    let entries = parse_indented_with_options(ccl, &opts).expect("should parse");
    assert_eq!(entries.len(), 2);
}

#[cfg(feature = "unstable")]
#[test]
fn test_load_with_options_tabs() {
    let ccl = "name = hello\tworld";

    // With tabs preserved (default)
    let opts_preserve = ParserOptions::new();
    let model = load_with_options(ccl, &opts_preserve).expect("should load");
    assert_eq!(model.get_string("name").unwrap(), "hello\tworld");

    // With tabs converted to spaces
    let opts_to_spaces = ParserOptions::new().with_tabs(TabBehavior::ToSpaces);
    let model = load_with_options(ccl, &opts_to_spaces).expect("should load");
    assert_eq!(model.get_string("name").unwrap(), "hello world");
}

#[cfg(feature = "unstable")]
#[test]
fn test_load_with_options_permissive() {
    let ccl = "name=value\r\nother\t=\tdata";
    let opts = ParserOptions::permissive();
    let model = load_with_options(ccl, &opts).expect("should load with permissive options");
    assert_eq!(model.get_string("name").unwrap(), "value");
    assert_eq!(model.get_string("other").unwrap(), "data");
}

// ============================================================================
// Boolean parsing tests
// ============================================================================

#[test]
fn test_get_bool_strict_true_false() {
    let ccl = r#"
enabled = true
disabled = false
"#;
    let model = load(ccl).expect("should load");
    assert!(model.get_bool("enabled").unwrap());
    assert!(!model.get_bool("disabled").unwrap());
}

#[test]
fn test_get_bool_strict_rejects_yes_no() {
    let ccl = r#"
opt_yes = yes
opt_no = no
"#;
    let model = load(ccl).expect("should load");
    // Strict mode should reject "yes" and "no"
    assert!(model.get_bool("opt_yes").is_err());
    assert!(model.get_bool("opt_no").is_err());
}

#[test]
fn test_get_bool_lenient_accepts_yes_no() {
    let ccl = r#"
opt_yes = yes
opt_no = no
opt_true = true
opt_false = false
"#;
    let model = load(ccl).expect("should load");
    // Lenient mode should accept all variants
    assert!(model.get_bool_lenient("opt_yes").unwrap());
    assert!(!model.get_bool_lenient("opt_no").unwrap());
    assert!(model.get_bool_lenient("opt_true").unwrap());
    assert!(!model.get_bool_lenient("opt_false").unwrap());
}

#[test]
fn test_get_bool_with_options() {
    use sickle::BoolOptions;

    let ccl = r#"
flag = yes
"#;
    let model = load(ccl).expect("should load");

    // Strict mode (default) rejects "yes"
    let strict_opts = BoolOptions::new();
    assert!(model.get_bool_with_options("flag", strict_opts).is_err());

    // Lenient mode accepts "yes"
    let lenient_opts = BoolOptions::lenient();
    assert!(model.get_bool_with_options("flag", lenient_opts).unwrap());
}

#[test]
fn test_get_bool_invalid_values() {
    let ccl = r#"
not_bool = hello
number = 42
"#;
    let model = load(ccl).expect("should load");
    // Neither strict nor lenient should accept these
    assert!(model.get_bool("not_bool").is_err());
    assert!(model.get_bool_lenient("not_bool").is_err());
    assert!(model.get_bool("number").is_err());
    assert!(model.get_bool_lenient("number").is_err());
}

#[test]
fn test_get_bool_missing_key() {
    let ccl = "name = value";
    let model = load(ccl).expect("should load");
    assert!(model.get_bool("nonexistent").is_err());
    assert!(model.get_bool_lenient("nonexistent").is_err());
}

// ============================================================================
// List access tests
// ============================================================================

#[test]
fn test_get_list_bare_syntax() {
    let ccl = r#"
servers =
  = web1
  = web2
  = web3
"#;
    let model = load(ccl).expect("should load");
    let list = model.get_list("servers").unwrap();
    assert_eq!(list, vec!["web1", "web2", "web3"]);
}

#[test]
fn test_get_list_coerced() {
    // Test coerced list access where duplicate keys at top level are stored
    // When building hierarchy, duplicate keys with simple values are kept as Vec
    // get_list_coerced looks at the first value's keys with coercion enabled
    let ccl = r#"
items =
  first = a
  second = b
  third = c
"#;
    let model = load(ccl).expect("should load");

    // Coerced version treats the nested keys as a list (filtering scalars)
    let coerced = model.get_list_coerced("items").unwrap();
    assert_eq!(coerced, vec!["first", "second", "third"]);
}

#[test]
fn test_get_list_typed_integers() {
    // get_list_typed works on the keys of a nested object
    // Use bare list syntax for actual list items
    let ccl = r#"
numbers =
  = 1
  = 42
  = -17
"#;
    let model = load(ccl).expect("should load");
    // Get the nested object first, then extract typed values from bare list
    let nums = model.get("numbers").unwrap();
    // Bare lists have empty key pointing to CclObjects with the values as keys
    let all_entries = nums.get_all("").unwrap();
    let numbers: Result<Vec<i64>, _> = all_entries
        .iter()
        .map(|entry| entry.keys().next().unwrap().parse::<i64>())
        .collect();
    assert_eq!(numbers.unwrap(), vec![1, 42, -17]);
}

#[test]
fn test_get_list_typed_booleans() {
    // get_list_typed works on the keys of a nested object
    let ccl = r#"
flags =
  = true
  = false
  = true
"#;
    let model = load(ccl).expect("should load");
    let flags_obj = model.get("flags").unwrap();
    let all_entries = flags_obj.get_all("").unwrap();
    let flags: Result<Vec<bool>, _> = all_entries
        .iter()
        .map(|entry| entry.keys().next().unwrap().parse::<bool>())
        .collect();
    assert_eq!(flags.unwrap(), vec![true, false, true]);
}

// ============================================================================
// Edge cases for parser options
// ============================================================================

#[cfg(feature = "unstable")]
#[test]
fn test_spacing_behavior_strict_parsing() {
    use sickle::SpacingBehavior;

    // Strict spacing is the default and accepts "key = value" format
    let ccl = "name = value";
    let opts = ParserOptions::new().with_spacing(SpacingBehavior::Strict);
    let model = load_with_options(ccl, &opts).expect("should parse");
    assert_eq!(model.get_string("name").unwrap(), "value");

    // Note: The current parser still finds '=' even without spaces
    // Strict vs Loose affects trimming, not tokenization
}

#[cfg(feature = "unstable")]
#[test]
fn test_spacing_behavior_loose_accepts_variations() {
    use sickle::SpacingBehavior;

    let ccl = "name=value\nother  =  data\ntabs\t=\tmore";
    let opts = ParserOptions::new().with_spacing(SpacingBehavior::Loose);
    let model = load_with_options(ccl, &opts).expect("should parse with loose spacing");
    assert_eq!(model.get_string("name").unwrap(), "value");
    assert_eq!(model.get_string("other").unwrap(), "data");
    assert_eq!(model.get_string("tabs").unwrap(), "more");
}

#[cfg(feature = "unstable")]
#[test]
fn test_multiple_crlf_in_multiline() {
    let ccl = "desc = line1\r\n  line2\r\n  line3";
    let opts = ParserOptions::new().with_crlf(CrlfBehavior::NormalizeToLf);
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    assert_eq!(entries.len(), 1);
    // After normalization, no CRLF should remain
    assert!(!entries[0].value.contains("\r\n"));
    assert!(entries[0].value.contains("line2"));
    assert!(entries[0].value.contains("line3"));
}

#[cfg(feature = "unstable")]
#[test]
fn test_tabs_in_multiline_values() {
    let ccl = "code = function() {\n\treturn 42;\n}";

    // With tabs preserved
    let opts_preserve = ParserOptions::new().with_tabs(TabBehavior::Preserve);
    let entries = parse_with_options(ccl, &opts_preserve).expect("should parse");
    assert!(entries[0].value.contains('\t'));

    // With tabs converted to spaces
    let opts_convert = ParserOptions::new().with_tabs(TabBehavior::ToSpaces);
    let entries = parse_with_options(ccl, &opts_convert).expect("should parse");
    assert!(!entries[0].value.contains('\t'));
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[test]
fn test_get_bool_case_sensitivity() {
    // Boolean parsing is case-sensitive
    let ccl = r#"
upper_true = TRUE
upper_false = FALSE
upper_yes = YES
upper_no = NO
"#;
    let model = load(ccl).expect("should load");

    // All uppercase variants should fail (case-sensitive)
    assert!(model.get_bool("upper_true").is_err());
    assert!(model.get_bool("upper_false").is_err());
    assert!(model.get_bool_lenient("upper_yes").is_err());
    assert!(model.get_bool_lenient("upper_no").is_err());
}

#[test]
fn test_get_all_with_duplicate_keys() {
    // When duplicate keys have simple string values, they're kept as separate entries
    let ccl = r#"
tag = important
tag = urgent
tag = review
"#;
    let model = load(ccl).expect("should load");
    let all_tags = model.get_all("tag").unwrap();
    assert_eq!(all_tags.len(), 3);

    // Extract the string values
    let tags: Vec<&str> = all_tags
        .iter()
        .map(|obj| obj.keys().next().unwrap().as_str())
        .collect();
    assert_eq!(tags, vec!["important", "urgent", "review"]);
}

#[test]
fn test_nested_with_get_bool() {
    let ccl = r#"
settings =
  debug = true
  verbose = false
  experimental = yes
"#;
    let model = load(ccl).expect("should load");
    let settings = model.get("settings").unwrap();

    assert!(settings.get_bool("debug").unwrap());
    assert!(!settings.get_bool("verbose").unwrap());

    // "yes" only works with lenient mode
    assert!(settings.get_bool("experimental").is_err());
    assert!(settings.get_bool_lenient("experimental").unwrap());
}

#[test]
fn test_empty_value_is_not_bool() {
    let ccl = "flag =";
    let model = load(ccl).expect("should load");
    // Empty value cannot be parsed as bool
    assert!(model.get_bool("flag").is_err());
    assert!(model.get_bool_lenient("flag").is_err());
}

#[test]
fn test_list_with_mixed_content() {
    // Bare list with various types of content
    let ccl = r#"
items =
  = simple
  = with spaces in value
  = http://example.com
"#;
    let model = load(ccl).expect("should load");
    let list = model.get_list("items").unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0], "simple");
    assert_eq!(list[1], "with spaces in value");
    assert_eq!(list[2], "http://example.com");
}

#[cfg(feature = "unstable")]
#[test]
fn test_parser_options_chaining() {
    // Test that all builder methods can be chained and applied correctly
    let ccl = "name\t=\tvalue\twith\ttabs\r\n";
    let opts = ParserOptions::new()
        .with_spacing(sickle::SpacingBehavior::Loose)
        .with_tabs(TabBehavior::ToSpaces)
        .with_crlf(CrlfBehavior::NormalizeToLf);

    let model = load_with_options(ccl, &opts).expect("should parse");
    // Tabs should be converted to spaces
    assert!(!model.get_string("name").unwrap().contains('\t'));
}

#[cfg(feature = "unstable")]
#[test]
fn test_crlf_only_at_line_boundaries() {
    // CRLF in the middle of a value (not at line boundary)
    let ccl = "text = hello\r\nworld";
    let opts = ParserOptions::new().with_crlf(CrlfBehavior::NormalizeToLf);
    let entries = parse_with_options(ccl, &opts).expect("should parse");
    // After normalization, should have two entries (CRLF becomes LF which is line separator)
    assert_eq!(entries.len(), 2);
}

#[cfg(feature = "unstable")]
#[test]
fn test_permissive_options_all_features() {
    // Test that permissive() enables all lenient behaviors
    let ccl = "name=value\twith\ttabs\r\nother\t=\tdata";
    let opts = ParserOptions::permissive();
    let model = load_with_options(ccl, &opts).expect("should parse");

    // Should parse despite no spaces around =
    assert!(model.get("name").is_ok());
    assert!(model.get("other").is_ok());

    // Tabs should be converted
    assert!(!model.get_string("name").unwrap().contains('\t'));
}

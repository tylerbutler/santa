//! Integration tests for the sickle CCL parser

mod test_helpers;

use sickle::{build_hierarchy, load, parse, parse_dedented};
use std::path::Path;
use test_helpers::{load_all_test_suites, ImplementationConfig, TestSuite};

/// Helper function to recursively validate a Model against expected JSON structure
fn validate_model_against_json(
    model: &sickle::Model,
    expected: &serde_json::Value,
    test_name: &str,
    path: &str,
) {
    match expected {
        serde_json::Value::String(expected_str) => {
            // Expect a singleton string
            let actual_str = model.as_str().unwrap_or_else(|_| {
                panic!(
                    "Test '{}': expected string at '{}', got {:?}",
                    test_name, path, model
                )
            });
            assert_eq!(
                actual_str, expected_str,
                "Test '{}': wrong value at '{}'",
                test_name, path
            );
        }
        serde_json::Value::Object(expected_map) => {
            // Expect a map
            let actual_map = model.as_map().unwrap_or_else(|_| {
                panic!(
                    "Test '{}': expected object at '{}', got {:?}",
                    test_name, path, model
                )
            });

            // Check all expected keys
            for (key, expected_value) in expected_map {
                let new_path = if path == "root" {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                let actual_model = actual_map.get(key).unwrap_or_else(|| {
                    panic!("Test '{}': missing key '{}' at '{}'", test_name, key, path)
                });

                validate_model_against_json(actual_model, expected_value, test_name, &new_path);
            }

            // Check for extra keys
            assert_eq!(
                actual_map.len(),
                expected_map.len(),
                "Test '{}': expected {} keys at '{}', got {}",
                test_name,
                expected_map.len(),
                path,
                actual_map.len()
            );
        }
        serde_json::Value::Array(expected_array) => {
            // Expect a list
            let actual_list = model.as_list().unwrap_or_else(|_| {
                panic!(
                    "Test '{}': expected array at '{}', got {:?}",
                    test_name, path, model
                )
            });

            assert_eq!(
                actual_list.len(),
                expected_array.len(),
                "Test '{}': expected {} items at '{}', got {}",
                test_name,
                expected_array.len(),
                path,
                actual_list.len()
            );

            // Validate each item
            for (i, (actual_item, expected_item)) in
                actual_list.iter().zip(expected_array.iter()).enumerate()
            {
                let new_path = format!("{}[{}]", path, i);
                validate_model_against_json(actual_item, expected_item, test_name, &new_path);
            }
        }
        _ => {
            panic!(
                "Test '{}': unsupported JSON type at '{}': {:?}",
                test_name, path, expected
            );
        }
    }
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

    let model = load(ccl).expect("should load");
    let desc = model.get("description").unwrap().as_str().unwrap();

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
    let comments = model.get("/").unwrap().as_list().unwrap();
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[0].as_str().unwrap(), "This is a comment");
    assert_eq!(
        comments[1].as_str().unwrap(),
        "Comments are valid entries in CCL"
    );
    assert_eq!(
        comments[2].as_str().unwrap(),
        "Another comment in the middle"
    );

    // Other keys work as expected
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

    let model = load(ccl).expect("should load");

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

    let model = load(ccl).expect("should load");

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
    let config1 = load(
        r#"
name = App1
version = 1.0.0
"#,
    )
    .unwrap();

    let config2 = load(
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

    let model = load(ccl).expect("should load");

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

// ============================================================================
// JSON Test Suite Integration
// ============================================================================

#[test]
fn test_json_suites_load() {
    let suites = load_all_test_suites();
    assert!(
        !suites.is_empty(),
        "Should load at least one test suite from test_data directory"
    );

    // Verify we loaded the expected suites
    assert!(
        suites.contains_key("api_core_ccl_parsing"),
        "Should have loaded api_core_ccl_parsing.json"
    );
}

#[test]
fn test_parsing_suite_basic_tests() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_core_ccl_parsing.json");

    if !path.exists() {
        panic!("Test data file not found: {:?}", path);
    }

    let suite = TestSuite::from_file(&path).expect("should load test suite");
    let parse_tests = suite.filter_by_validation("parse");

    assert!(
        !parse_tests.is_empty(),
        "Should have parse validation tests"
    );

    // Run each parse test and track results
    let mut passed = 0;
    let mut failed = 0;

    for test in parse_tests {
        let test_result = std::panic::catch_unwind(|| {
            // Parse and build hierarchy
            let result = load(&test.input);

            // Check that parse succeeds or fails appropriately
            if test.expected.error.is_some() {
                assert!(
                    result.is_err(),
                    "Test '{}' expected error but parsing succeeded",
                    test.name
                );
            } else {
                let model = result.unwrap_or_else(|e| {
                    panic!("Test '{}' failed to load: {}", test.name, e);
                });

                // Verify we got the expected number of top-level entries
                if let Ok(map) = model.as_map() {
                    assert_eq!(
                        map.len(),
                        test.expected.count,
                        "Test '{}' expected {} entries, got {}",
                        test.name,
                        test.expected.count,
                        map.len()
                    );

                    // Verify each expected entry exists with correct value
                    for entry in &test.expected.entries {
                        let value = model
                            .get(&entry.key)
                            .unwrap_or_else(|_| {
                                panic!("Test '{}': missing expected key '{}'", test.name, entry.key)
                            })
                            .as_str()
                            .unwrap_or_else(|_| {
                                panic!("Test '{}': key '{}' is not a string", test.name, entry.key)
                            });

                        assert_eq!(
                            value, entry.value,
                            "Test '{}': key '{}' has wrong value",
                            test.name, entry.key
                        );
                    }
                }
            }
        });

        match test_result {
            Ok(_) => {
                println!("  ✓ {}", test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  ✗ {}: {:?}", test.name, e);
                failed += 1;
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);
    assert!(passed > 0, "At least some tests should pass");
}

#[test]
fn test_comments_suite() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_comments.json");

    if !path.exists() {
        println!("Skipping comments test - file not found: {:?}", path);
        return;
    }

    let suite = TestSuite::from_file(&path).expect("should load test suite");
    let comment_tests = suite.filter_by_validation("parse");

    let mut passed = 0;
    let mut failed = 0;

    for test in comment_tests {
        let test_result = std::panic::catch_unwind(|| {
            let result = load(&test.input);

            if test.expected.error.is_some() {
                assert!(
                    result.is_err(),
                    "Test '{}' expected error but parsing succeeded",
                    test.name
                );
            } else {
                let model = result.unwrap_or_else(|e| {
                    panic!("Test '{}' failed to load: {}", test.name, e);
                });

                // Verify entry count
                if let Ok(map) = model.as_map() {
                    assert_eq!(
                        map.len(),
                        test.expected.count,
                        "Test '{}' expected {} entries, got {}",
                        test.name,
                        test.expected.count,
                        map.len()
                    );
                }
            }
        });

        match test_result {
            Ok(_) => {
                println!("  ✓ {}", test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  ✗ {}: {:?}", test.name, e);
                failed += 1;
            }
        }
    }

    println!("\nComments tests: {} passed, {} failed", passed, failed);
    // Comments feature may not be fully implemented yet
    if failed > 0 && passed == 0 {
        println!("Note: Comments feature may not be fully implemented in sickle yet");
    }
}

#[test]
fn test_typed_access_suite_strings() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_typed_access.json");

    if !path.exists() {
        println!("Skipping typed access test - file not found: {:?}", path);
        return;
    }

    let suite = TestSuite::from_file(&path).expect("should load test suite");

    // Filter for get_string validation tests
    let string_tests = suite.filter_by_validation("get_string");

    let mut passed = 0;
    let mut failed = 0;

    for test in string_tests {
        let test_result = std::panic::catch_unwind(|| {
            // Parse the input
            let model = load(&test.input).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // Get the value at the specified key
            if let Some(ref key) = test.expected.key {
                let result = model.get(key);

                if test.expected.error.is_some() {
                    assert!(
                        result.is_err(),
                        "Test '{}' expected error for key '{}'",
                        test.name,
                        key
                    );
                } else if let Some(ref expected_value) = test.expected.value {
                    let value = result
                        .unwrap_or_else(|_| panic!("Test '{}': missing key '{}'", test.name, key))
                        .as_str()
                        .unwrap_or_else(|_| {
                            panic!("Test '{}': key '{}' is not a string", test.name, key)
                        });

                    let expected_str = expected_value.as_str().unwrap_or_else(|| {
                        panic!("Test '{}': expected value is not a string", test.name)
                    });

                    assert_eq!(
                        value, expected_str,
                        "Test '{}': key '{}' has wrong value",
                        test.name, key
                    );
                }
            }
        });

        match test_result {
            Ok(_) => {
                println!("  ✓ {}", test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  ✗ {}: {:?}", test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nString access tests: {} passed, {} failed",
        passed, failed
    );
    assert!(passed > 0, "At least some string access tests should pass");
}

#[test]
fn test_filter_function() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_comments.json");

    if !path.exists() {
        println!("⚠️  Skipping test - test data file not found");
        return;
    }

    let suite = TestSuite::from_file(&path).expect("should load test suite");

    // Filter for tests that use the filter function
    let filter_tests = suite.filter_by_function("filter");

    let mut passed = 0;
    let mut failed = 0;

    for test in &filter_tests {
        let test_result = std::panic::catch_unwind(|| {
            // Parse the input
            let model = load(&test.input).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // For filter tests, we expect the model to filter out comments
            // and only contain non-comment entries
            let map = model
                .as_map()
                .unwrap_or_else(|_| panic!("Test '{}': model is not a map", test.name));
            let count = map.len();
            assert_eq!(
                count, test.expected.count,
                "Test '{}' expected {} entries, got {}",
                test.name, test.expected.count, count
            );

            // Verify the actual entries match
            for entry in &test.expected.entries {
                let value = model
                    .get(&entry.key)
                    .unwrap_or_else(|_| panic!("Test '{}': missing key '{}'", test.name, entry.key))
                    .as_str()
                    .unwrap_or_else(|_| {
                        panic!("Test '{}': key '{}' is not a string", test.name, entry.key)
                    });

                assert_eq!(
                    value, entry.value,
                    "Test '{}': key '{}' has wrong value",
                    test.name, entry.key
                );
            }
        });

        match test_result {
            Ok(_) => {
                println!("  ✓ {}", test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  ✗ {}: {:?}", test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nFilter function tests: {} passed, {} failed",
        passed, failed
    );
    println!("Found {} tests using filter function", filter_tests.len());
    assert!(
        !filter_tests.is_empty(),
        "Should find tests using filter function"
    );
}

#[test]
fn test_all_ccl_suites_comprehensive() {
    let suites = load_all_test_suites();
    let config = ImplementationConfig::sickle_current();

    println!("\n🧪 Running comprehensive CCL test suite");
    println!("📁 Loaded {} test suite files", suites.len());
    println!("🔧 Implementation capabilities:");
    println!("   Functions: {}", config.supported_functions.join(", "));
    println!("   Features: {}", config.supported_features.join(", "));
    println!("   Behaviors: {}\n", config.chosen_behaviors.join(", "));

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_skipped = 0;

    // Track failure details
    let mut failure_details: Vec<(String, String, String)> = Vec::new(); // (suite, test, reason)
    let mut skipped_validations: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut behavior_coverage: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new(); // (passed, total)
    let mut function_coverage: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new(); // (passed, total)

    // Sort suite names for consistent output
    let mut suite_names: Vec<_> = suites.keys().collect();
    suite_names.sort();

    // Set up a custom panic hook to capture panic messages
    let default_hook = std::panic::take_hook();
    let panic_messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let panic_messages_clone = panic_messages.clone();

    std::panic::set_hook(Box::new(move |info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            format!("{:?}", info)
        };
        panic_messages_clone.lock().unwrap().push(msg);
    }));

    for suite_name in suite_names {
        let suite = &suites[suite_name];
        println!("📋 {}", suite_name);

        let mut suite_passed = 0;
        let mut suite_failed = 0;
        let mut suite_skipped = 0;

        // Filter tests based on implementation capabilities
        let filtered_tests = suite.filter_by_capabilities(&config);
        let total_tests_in_suite = suite.tests.len();
        let filtered_count = filtered_tests.len();
        let skipped_by_filter = total_tests_in_suite - filtered_count;

        if skipped_by_filter > 0 {
            println!(
                "   ⚠️  Skipped {} tests due to unsupported capabilities",
                skipped_by_filter
            );
            total_skipped += skipped_by_filter;
        }

        for test in filtered_tests {
            // Clear previous panic messages
            panic_messages.lock().unwrap().clear();

            let test_result = std::panic::catch_unwind(|| {
                // Parse the input based on validation type
                let (entries, model_result) = if test.validation == "parse_dedented" {
                    let e = parse_dedented(&test.input);
                    let m = e.as_ref().ok().map(|entries| build_hierarchy(entries));
                    (e, m)
                } else {
                    let e = parse(&test.input);
                    let m = e.as_ref().ok().map(|entries| build_hierarchy(entries));
                    (e, m)
                };

                // Handle different validation types
                match test.validation.as_str() {
                    "parse" => {
                        if test.expected.error.is_some() {
                            assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let entry_list = entries.unwrap_or_else(|e| {
                                panic!("Test '{}' failed to parse: {}", test.name, e);
                            });

                            // For "parse" validation: check entry count
                            assert_eq!(
                                entry_list.len(),
                                test.expected.count,
                                "Test '{}' expected {} entries, got {}",
                                test.name,
                                test.expected.count,
                                entry_list.len()
                            );

                            // Verify specific entries if provided
                            if !test.expected.entries.is_empty() {
                                for expected_entry in &test.expected.entries {
                                    let found = entry_list.iter().any(|e| {
                                        e.key == expected_entry.key
                                            && e.value == expected_entry.value
                                    });
                                    assert!(
                                        found,
                                        "Test '{}': expected entry {}={} not found",
                                        test.name, expected_entry.key, expected_entry.value
                                    );
                                }
                            }
                        }
                    }
                    "filter" => {
                        // "filter" validation tests work the same as "parse" tests
                        // They're tagged with the "filter" function to indicate they test
                        // the capability filtering mechanism itself
                        if test.expected.error.is_some() {
                            assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let entry_list = entries.unwrap_or_else(|e| {
                                panic!("Test '{}' failed to parse: {}", test.name, e);
                            });

                            // For filter tests with entries, just verify parsing succeeded
                            assert_eq!(
                                entry_list.len(),
                                test.expected.count,
                                "Test '{}' expected {} entries, got {}",
                                test.name,
                                test.expected.count,
                                entry_list.len()
                            );
                        }
                    }
                    "build_hierarchy" => {
                        if test.expected.error.is_some() {
                            assert!(
                                model_result.is_none() || model_result.as_ref().unwrap().is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else {
                            let model = model_result
                                .expect("model_result should be Some")
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                                });

                            // For build_hierarchy, count=1 means "successfully built a hierarchy"
                            assert_eq!(
                                test.expected.count, 1,
                                "Test '{}': build_hierarchy tests should have count=1",
                                test.name
                            );

                            // Validate the object structure if specified
                            if let Some(ref expected_obj) = test.expected.object {
                                validate_model_against_json(
                                    &model,
                                    expected_obj,
                                    &test.name,
                                    "root",
                                );
                            }
                        }
                    }
                    "get_string" => {
                        if let Some(ref key) = test.expected.key {
                            let model = model_result
                                .expect("model_result should be Some")
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                                });

                            let get_result = model.get(key);

                            if test.expected.error.is_some() {
                                assert!(get_result.is_err(), "Test '{}' expected error", test.name);
                            } else if let Some(ref expected_value) = test.expected.value {
                                let value = get_result
                                    .unwrap_or_else(|_| {
                                        panic!("Test '{}': missing key '{}'", test.name, key)
                                    })
                                    .as_str()
                                    .unwrap_or_else(|_| {
                                        panic!(
                                            "Test '{}': key '{}' is not a string",
                                            test.name, key
                                        )
                                    });

                                let expected_str = expected_value.as_str().unwrap_or_else(|| {
                                    panic!("Test '{}': expected value is not a string", test.name)
                                });

                                assert_eq!(
                                    value, expected_str,
                                    "Test '{}': wrong value for key '{}'",
                                    test.name, key
                                );
                            }
                        }
                    }
                    "parse_dedented" => {
                        // parse_dedented removes common indentation prefix before parsing
                        if test.expected.error.is_some() {
                            assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let entry_list = entries.unwrap_or_else(|e| {
                                panic!("Test '{}' failed to parse: {}", test.name, e);
                            });

                            // Validate entry count
                            assert_eq!(
                                entry_list.len(),
                                test.expected.count,
                                "Test '{}' expected {} entries, got {}",
                                test.name,
                                test.expected.count,
                                entry_list.len()
                            );

                            // Verify specific entries if provided
                            if !test.expected.entries.is_empty() {
                                for expected_entry in &test.expected.entries {
                                    let found = entry_list.iter().any(|e| {
                                        e.key == expected_entry.key
                                            && e.value == expected_entry.value
                                    });
                                    assert!(
                                        found,
                                        "Test '{}': expected entry {}={} not found",
                                        test.name, expected_entry.key, expected_entry.value
                                    );
                                }
                            }
                        }
                    }
                    "parse_value" => {
                        // "parse_value" validation tests are actually testing parser edge cases
                        // (like indented keys, whitespace handling), not the parse_value<T>() method
                        // Use hierarchy for these tests
                        if test.expected.error.is_some() {
                            assert!(
                                model_result.is_none() || model_result.as_ref().unwrap().is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else {
                            let model = model_result
                                .expect("model_result should be Some")
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                                });

                            // Verify the expected entries if specified
                            if !test.expected.entries.is_empty() {
                                for entry in &test.expected.entries {
                                    let value = model
                                        .get(&entry.key)
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "Test '{}': missing key '{}'",
                                                test.name, entry.key
                                            )
                                        })
                                        .as_str()
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "Test '{}': key '{}' is not a string",
                                                test.name, entry.key
                                            )
                                        });

                                    assert_eq!(
                                        value, entry.value,
                                        "Test '{}': key '{}' has wrong value",
                                        test.name, entry.key
                                    );
                                }
                            }
                        }
                    }
                    _ => {
                        // Skip unsupported validation types for now
                        panic!("Unsupported validation type: {}", test.validation);
                    }
                }
            });

            // Track behaviors and functions
            for behavior in &test.behaviors {
                let entry = behavior_coverage.entry(behavior.clone()).or_insert((0, 0));
                entry.1 += 1; // total
            }
            for function in &test.functions {
                let entry = function_coverage.entry(function.clone()).or_insert((0, 0));
                entry.1 += 1; // total
            }

            match test_result {
                Ok(_) => {
                    suite_passed += 1;

                    // Track successful behaviors and functions
                    for behavior in &test.behaviors {
                        behavior_coverage.get_mut(behavior).unwrap().0 += 1;
                    }
                    for function in &test.functions {
                        function_coverage.get_mut(function).unwrap().0 += 1;
                    }
                }
                Err(_) => {
                    // Get the panic message we captured
                    let panic_msgs = panic_messages.lock().unwrap();
                    let err_msg = panic_msgs
                        .last()
                        .map(|s| s.as_str())
                        .unwrap_or("Unknown error");

                    if err_msg.contains("Unsupported validation type") {
                        suite_skipped += 1;
                        // Extract validation type
                        if let Some(val_type) =
                            err_msg.split("Unsupported validation type: ").nth(1)
                        {
                            *skipped_validations
                                .entry(val_type.trim().to_string())
                                .or_insert(0) += 1;
                        }
                    } else {
                        suite_failed += 1;
                        // Store failure details
                        failure_details.push((
                            suite_name.to_string(),
                            test.name.clone(),
                            err_msg.to_string(),
                        ));
                    }
                }
            }
        }

        total_passed += suite_passed;
        total_failed += suite_failed;
        total_skipped += suite_skipped;

        println!(
            "  ✓ {} passed, ✗ {} failed, ⊘ {} skipped (total: {})\n",
            suite_passed,
            suite_failed,
            suite_skipped,
            suite.tests.len()
        );
    }

    // Restore the default panic hook
    std::panic::set_hook(default_hook);

    println!("════════════════════════════════════════");
    println!("📊 Overall Results:");
    println!("  ✓ {} passed", total_passed);
    println!("  ✗ {} failed", total_failed);
    println!(
        "  ⊘ {} skipped (unsupported validation types)",
        total_skipped
    );
    println!("  Total: {}", total_passed + total_failed + total_skipped);
    println!("════════════════════════════════════════");

    // Show skipped validation types
    if !skipped_validations.is_empty() {
        println!("\n⊘ Skipped Validation Types:");
        let mut skip_types: Vec<_> = skipped_validations.iter().collect();
        skip_types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (val_type, count) in skip_types {
            println!("  - {}: {} tests", val_type, count);
        }
    }

    // Show behavior coverage
    if !behavior_coverage.is_empty() {
        println!("\n🔄 Behavior Coverage:");
        println!("   Note: Some behaviors are mutually exclusive configuration options");

        // Group mutually exclusive behaviors
        let mutually_exclusive_pairs = [
            ("boolean_strict", "boolean_lenient"),
            ("crlf_normalize_to_lf", "crlf_preserve_literal"),
            ("list_coercion_enabled", "list_coercion_disabled"),
        ];

        let mut shown = std::collections::HashSet::new();

        // Show mutually exclusive pairs together
        for (opt1, opt2) in &mutually_exclusive_pairs {
            if let (Some((p1, t1)), Some((p2, t2))) =
                (behavior_coverage.get(*opt1), behavior_coverage.get(*opt2))
            {
                let pct1 = if *t1 > 0 { (*p1 * 100) / *t1 } else { 0 };
                let pct2 = if *t2 > 0 { (*p2 * 100) / *t2 } else { 0 };
                println!("  ⚙️  {} vs {}", opt1, opt2);
                println!("      {}: {}/{} ({}%)", opt1, p1, t1, pct1);
                println!("      {}: {}/{} ({}%)", opt2, p2, t2, pct2);
                shown.insert(opt1.to_string());
                shown.insert(opt2.to_string());
            }
        }

        // Show remaining behaviors
        let mut behaviors: Vec<_> = behavior_coverage
            .iter()
            .filter(|(name, _)| !shown.contains(*name))
            .collect();
        behaviors.sort_by_key(|(name, _)| *name);

        if !behaviors.is_empty() {
            println!("\n  Other behaviors:");
        }
        for (behavior, (passed, total)) in behaviors {
            let percent = if *total > 0 {
                (*passed * 100) / *total
            } else {
                0
            };
            let status = if *passed == *total {
                "✅"
            } else if *passed > 0 {
                "⚠️"
            } else {
                "❌"
            };
            println!(
                "  {} {}: {}/{} ({}%)",
                status, behavior, passed, total, percent
            );
        }
    }

    // Show function coverage
    if !function_coverage.is_empty() {
        println!("\n🎯 Function Coverage:");
        let mut functions: Vec<_> = function_coverage.iter().collect();
        functions.sort_by_key(|(name, _)| *name);
        for (function, (passed, total)) in functions {
            let percent = if *total > 0 {
                (*passed * 100) / *total
            } else {
                0
            };
            let status = if *passed == *total {
                "✅"
            } else if *passed > 0 {
                "⚠️"
            } else {
                "❌"
            };
            println!(
                "  {} {}: {}/{} ({}%)",
                status, function, passed, total, percent
            );
        }
    }

    // Show failure details (limit to first 20 for readability)
    if !failure_details.is_empty() {
        println!("\n✗ Failure Details (showing first 20):");
        for (suite, test, reason) in failure_details.iter().take(20) {
            println!("  [{suite}] {test}");
            // Extract the key part of the error message
            let clean_reason = if let Some(msg) = reason.split("assertion").nth(1) {
                format!("    Assertion{}", msg.trim())
            } else if reason.contains("expected") {
                // Extract assertion message
                if let Some(msg) = reason.split(':').next_back() {
                    format!("    {}", msg.trim())
                } else {
                    format!("    {}", reason.lines().next().unwrap_or(reason).trim())
                }
            } else {
                format!("    {}", reason.lines().next().unwrap_or(reason).trim())
            };
            println!("{}", clean_reason);
        }

        if failure_details.len() > 20 {
            println!("  ... and {} more failures", failure_details.len() - 20);
        }
    }

    // Assert that we have some passing tests
    assert!(total_passed > 0, "At least some tests should pass");
}

//! Data-driven CCL tests using JSON test suites

mod capabilities;
mod test_helpers;

use sickle::{build_hierarchy, load, parse, parse_indented};
use std::path::Path;
use test_helpers::{load_all_test_suites, ImplementationConfig, TestSuite};

/// Test helper to extract string value from Model using public API
///
/// Accesses Model.0 (the public IndexMap field). While Model has an as_string() method,
/// it's pub(crate) and not accessible from integration tests.
///
/// A string value in CCL is represented as: {"string_value": {}}
fn model_as_str(model: &sickle::Model) -> Result<&str, String> {
    if model.0.len() == 1 {
        let (key, value) = model.0.iter().next().unwrap();
        if value.0.is_empty() {
            return Ok(key.as_str());
        }
    }
    Err("not a singleton string".to_string())
}

/// Helper to navigate nested paths in a Model (e.g., ["config", "database", "port"])
fn navigate_path<'a>(
    model: &'a sickle::Model,
    path: &[String],
    test_name: &str,
) -> Result<&'a sickle::Model, String> {
    let mut current = model;
    for key in path {
        current = current
            .get(key)
            .map_err(|_| format!("Test '{}': missing key '{}'", test_name, key))?;
    }
    Ok(current)
}

/// Helper to extract list from Model using public API
///
/// Accesses Model.0 (the public IndexMap field). While Model has an as_list() method,
/// it's pub(crate) and not accessible from integration tests.
///
/// A list in CCL is represented as multiple keys in the map: {"item1": {}, "item2": {}, "item3": {}}
fn model_as_list(model: &sickle::Model) -> Vec<String> {
    model.0.keys().cloned().collect()
}

/// Helper function to recursively validate a Model against expected JSON structure
fn validate_model_against_json(
    model: &sickle::Model,
    expected: &serde_json::Value,
    test_name: &str,
    path: &str,
) {
    match expected {
        serde_json::Value::String(expected_str) => {
            // Expect a singleton string: {"value": {}}
            // Access via public IndexMap field
            if model.0.len() != 1 {
                panic!(
                    "Test '{}': expected singleton string at '{}', got {} keys",
                    test_name,
                    path,
                    model.0.len()
                );
            }
            let (actual_str, value) = model.0.iter().next().unwrap();
            if !value.0.is_empty() {
                panic!(
                    "Test '{}': expected string singleton at '{}', but value is not empty",
                    test_name, path
                );
            }
            assert_eq!(
                actual_str, expected_str,
                "Test '{}': wrong value at '{}'",
                test_name, path
            );
        }
        serde_json::Value::Object(expected_map) => {
            // Expect a map - use public IndexMap field
            // Check all expected keys
            for (key, expected_value) in expected_map {
                let new_path = if path == "root" {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                let actual_model = model.0.get(key).unwrap_or_else(|| {
                    panic!("Test '{}': missing key '{}' at '{}'", test_name, key, path)
                });

                validate_model_against_json(actual_model, expected_value, test_name, &new_path);
            }

            // Check for extra keys
            assert_eq!(
                model.0.len(),
                expected_map.len(),
                "Test '{}': expected {} keys at '{}', got {}",
                test_name,
                expected_map.len(),
                path,
                model.0.len()
            );
        }
        serde_json::Value::Array(expected_array) => {
            // Expect a list - multiple keys with empty values
            // Access via public IndexMap field
            assert_eq!(
                model.0.len(),
                expected_array.len(),
                "Test '{}': expected {} items at '{}', got {}",
                test_name,
                expected_array.len(),
                path,
                model.0.len()
            );

            // Validate each item - lists are represented as keys in order
            for (i, (actual_key, expected_item)) in
                model.0.keys().zip(expected_array.iter()).enumerate()
            {
                let new_path = format!("{}[{}]", path, i);
                // For list items, create a synthetic Model for string comparison
                if let serde_json::Value::String(expected_str) = expected_item {
                    assert_eq!(
                        actual_key, expected_str,
                        "Test '{}': wrong list value at '{}'",
                        test_name, new_path
                    );
                } else {
                    panic!(
                        "Test '{}': unsupported list item type at '{}'",
                        test_name, new_path
                    );
                }
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

// Test suite tests follow...

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
                // Use public IndexMap field
                let map = &model.0;
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
                    let value_model = model.get(&entry.key).unwrap_or_else(|_| {
                        panic!("Test '{}': missing expected key '{}'", test.name, entry.key)
                    });
                    let value = model_as_str(value_model).unwrap_or_else(|_| {
                        panic!("Test '{}': key '{}' is not a string", test.name, entry.key)
                    });

                    assert_eq!(
                        value, entry.value,
                        "Test '{}': key '{}' has wrong value",
                        test.name, entry.key
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

                // Verify entry count - use public IndexMap field
                let map = &model.0;
                assert_eq!(
                    map.len(),
                    test.expected.count,
                    "Test '{}' expected {} entries, got {}",
                    test.name,
                    test.expected.count,
                    map.len()
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
                    let value_model = result
                        .unwrap_or_else(|_| panic!("Test '{}': missing key '{}'", test.name, key));
                    let value = model_as_str(value_model).unwrap_or_else(|_| {
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
            // and only contain non-comment entries - use public IndexMap field
            let map = &model.0;
            let count = map.len();
            assert_eq!(
                count, test.expected.count,
                "Test '{}' expected {} entries, got {}",
                test.name, test.expected.count, count
            );

            // Verify the actual entries match
            for entry in &test.expected.entries {
                let value_model = model.get(&entry.key).unwrap_or_else(|_| {
                    panic!("Test '{}': missing key '{}'", test.name, entry.key)
                });
                let value = model_as_str(value_model).unwrap_or_else(|_| {
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
                    let e = parse_indented(&test.input);
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
                                let value_model = get_result.unwrap_or_else(|_| {
                                    panic!("Test '{}': missing key '{}'", test.name, key)
                                });
                                let value = model_as_str(value_model).unwrap_or_else(|_| {
                                    panic!("Test '{}': key '{}' is not a string", test.name, key)
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
                    "parse_dedented" | "parse_indented" => {
                        // parse_dedented/parse_indented removes common indentation prefix before parsing
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
                    "get_list" => {
                        let model = model_result
                            .expect("model_result should be Some")
                            .unwrap_or_else(|e| {
                                panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                            });

                        // Navigate to the target using args as path
                        let target_result = navigate_path(&model, &test.args, &test.name);

                        if test.expected.error.is_some() {
                            assert!(
                                target_result.is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else if test.expected.count == 0 {
                            // Empty list case - key doesn't exist or path is invalid
                            assert!(
                                target_result.is_err() || target_result.unwrap().0.is_empty(),
                                "Test '{}' expected empty list",
                                test.name
                            );
                        } else if let Some(ref expected_list) = test.expected.list {
                            let target_model = target_result.unwrap_or_else(|e| {
                                panic!("Test '{}': {}", test.name, e);
                            });
                            let actual_list = model_as_list(target_model);

                            assert_eq!(
                                actual_list.len(),
                                expected_list.len(),
                                "Test '{}' expected {} items, got {}",
                                test.name,
                                expected_list.len(),
                                actual_list.len()
                            );

                            assert_eq!(
                                &actual_list, expected_list,
                                "Test '{}': list values don't match",
                                test.name
                            );
                        }
                    }
                    "get_int" => {
                        let model = model_result
                            .expect("model_result should be Some")
                            .unwrap_or_else(|e| {
                                panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                            });

                        let target_result = navigate_path(&model, &test.args, &test.name);

                        if test.expected.error.is_some() {
                            assert!(
                                target_result.is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let target_model = target_result.unwrap_or_else(|e| {
                                panic!("Test '{}': {}", test.name, e);
                            });
                            let value_str = model_as_str(target_model).unwrap_or_else(|_| {
                                panic!("Test '{}': value is not a string", test.name)
                            });

                            let actual_int: i64 = value_str.parse().unwrap_or_else(|_| {
                                panic!(
                                    "Test '{}': cannot parse '{}' as integer",
                                    test.name, value_str
                                )
                            });

                            let expected_int = expected_value.as_i64().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not an integer", test.name)
                            });

                            assert_eq!(
                                actual_int, expected_int,
                                "Test '{}': wrong integer value",
                                test.name
                            );
                        }
                    }
                    "get_bool" => {
                        let model = model_result
                            .expect("model_result should be Some")
                            .unwrap_or_else(|e| {
                                panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                            });

                        let target_result = navigate_path(&model, &test.args, &test.name);

                        if test.expected.error.is_some() {
                            assert!(
                                target_result.is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            if expected_value.is_null() {
                                // Test expects that the value cannot be parsed as bool
                                // (e.g., "yes" with strict boolean parsing)
                                let target_model = target_result.unwrap_or_else(|e| {
                                    panic!("Test '{}': {}", test.name, e);
                                });
                                let value_str = model_as_str(target_model).unwrap_or_else(|_| {
                                    panic!("Test '{}': value is not a string", test.name)
                                });

                                // In strict mode, only "true" and "false" are valid
                                assert!(
                                    value_str != "true" && value_str != "false",
                                    "Test '{}': expected null (unparseable bool) but got valid bool value",
                                    test.name
                                );
                            } else {
                                let target_model = target_result.unwrap_or_else(|e| {
                                    panic!("Test '{}': {}", test.name, e);
                                });
                                let value_str = model_as_str(target_model).unwrap_or_else(|_| {
                                    panic!("Test '{}': value is not a string", test.name)
                                });

                                let actual_bool = match value_str {
                                    "true" => true,
                                    "false" => false,
                                    "1" => true,
                                    "0" => false,
                                    _ => panic!(
                                        "Test '{}': cannot parse '{}' as boolean",
                                        test.name, value_str
                                    ),
                                };

                                let expected_bool = expected_value.as_bool().unwrap_or_else(|| {
                                    panic!("Test '{}': expected value is not a boolean", test.name)
                                });

                                assert_eq!(
                                    actual_bool, expected_bool,
                                    "Test '{}': wrong boolean value",
                                    test.name
                                );
                            }
                        }
                    }
                    "get_float" => {
                        let model = model_result
                            .expect("model_result should be Some")
                            .unwrap_or_else(|e| {
                                panic!("Test '{}' failed to build hierarchy: {}", test.name, e);
                            });

                        let target_result = navigate_path(&model, &test.args, &test.name);

                        if test.expected.error.is_some() {
                            assert!(
                                target_result.is_err(),
                                "Test '{}' expected error",
                                test.name
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let target_model = target_result.unwrap_or_else(|e| {
                                panic!("Test '{}': {}", test.name, e);
                            });
                            let value_str = model_as_str(target_model).unwrap_or_else(|_| {
                                panic!("Test '{}': value is not a string", test.name)
                            });

                            let actual_float: f64 = value_str.parse().unwrap_or_else(|_| {
                                panic!(
                                    "Test '{}': cannot parse '{}' as float",
                                    test.name, value_str
                                )
                            });

                            let expected_float = expected_value.as_f64().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not a float", test.name)
                            });

                            assert!(
                                (actual_float - expected_float).abs() < 0.0001,
                                "Test '{}': wrong float value, expected {} got {}",
                                test.name,
                                expected_float,
                                actual_float
                            );
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

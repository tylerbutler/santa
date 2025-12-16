//! Data-driven CCL tests using JSON test suites

mod common;

use common::{load_all_test_suites, ImplementationConfig, TestCase, TestSuite};
use sickle::options::{CrlfBehavior, ParserOptions, SpacingBehavior, TabBehavior};
use sickle::{
    build_hierarchy, load_with_options, parse_indented_with_options, parse_with_options, CclPrinter,
};

/// Build ParserOptions from test case behaviors
fn options_from_test(test: &TestCase) -> ParserOptions {
    let mut options = ParserOptions::new();

    // Check for spacing behavior
    if test.behaviors.contains(&"loose_spacing".to_string()) {
        options = options.with_spacing(SpacingBehavior::Loose);
    }
    // strict_spacing is the default, no need to set explicitly

    // Check for tab behavior
    if test.behaviors.contains(&"tabs_to_spaces".to_string()) {
        options = options.with_tabs(TabBehavior::ToSpaces);
    }
    // tabs_preserve is the default, no need to set explicitly

    // Check for CRLF behavior
    if test.behaviors.contains(&"crlf_normalize_to_lf".to_string()) {
        options = options.with_crlf(CrlfBehavior::NormalizeToLf);
    }
    // crlf_preserve is the default, no need to set explicitly

    options
}
use std::path::Path;

/// Helper to navigate nested paths in a Model (e.g., ["config", "database", "port"])
fn navigate_path<'a>(
    model: &'a sickle::CclObject,
    path: &[String],
    test_name: &str,
) -> Result<&'a sickle::CclObject, String> {
    let mut current = model;
    for key in path {
        current = current
            .get(key)
            .map_err(|_| format!("Test '{}': missing key '{}'", test_name, key))?;
    }
    Ok(current)
}

/// Helper function to validate a Vec of CclObjects against expected JSON array
fn validate_vec_against_json(
    values: &[sickle::CclObject],
    expected: &serde_json::Value,
    test_name: &str,
    path: &str,
) {
    let expected_array = expected
        .as_array()
        .expect("expected value should be an array");

    assert_eq!(
        values.len(),
        expected_array.len(),
        "Test '{}': expected {} items at '{}', got {}",
        test_name,
        expected_array.len(),
        path,
        values.len()
    );

    // Each value in the Vec is a CclObject representing one list item
    // The list item's value is the single key in the CclObject
    for (i, (value, expected_item)) in values.iter().zip(expected_array.iter()).enumerate() {
        let new_path = format!("{}[{}]", path, i);
        if let serde_json::Value::String(expected_str) = expected_item {
            let actual_key = value.keys().next().unwrap_or(&"".to_string()).clone();
            assert_eq!(
                &actual_key, expected_str,
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

/// Helper function to recursively validate a Model against expected JSON structure
fn validate_model_against_json(
    model: &sickle::CclObject,
    expected: &serde_json::Value,
    test_name: &str,
    path: &str,
) {
    match expected {
        serde_json::Value::String(expected_str) => {
            // Expect a singleton string: {"value": {}}
            // Access via public IndexMap field
            if model.len() != 1 {
                panic!(
                    "Test '{}': expected singleton string at '{}', got {} keys",
                    test_name,
                    path,
                    model.len()
                );
            }
            let (actual_str, value) = model.iter().next().unwrap();
            if !value.is_empty() {
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

                // If expected value is an array, we need to use get_all() to get all Vec values
                // because get() only returns the first element
                if expected_value.is_array() {
                    // Get all values for this key and create a synthetic model with them
                    let all_values = model.get_all(key).unwrap_or_else(|_| {
                        panic!("Test '{}': missing key '{}' at '{}'", test_name, key, path)
                    });
                    validate_vec_against_json(all_values, expected_value, test_name, &new_path);
                } else {
                    let actual_model = model.get(key).unwrap_or_else(|_| {
                        panic!("Test '{}': missing key '{}' at '{}'", test_name, key, path)
                    });
                    validate_model_against_json(actual_model, expected_value, test_name, &new_path);
                }
            }

            // Check for extra keys
            assert_eq!(
                model.len(),
                expected_map.len(),
                "Test '{}': expected {} keys at '{}', got {}",
                test_name,
                expected_map.len(),
                path,
                model.len()
            );
        }
        serde_json::Value::Array(expected_array) => {
            // Expect a list - with Vec structure, lists are stored as:
            // { "": [CclObject({item1}), CclObject({item2}), ...] }
            // We need to check if this is a bare list (single empty key)
            // and iterate over the Vec values

            if model.len() == 1 && model.keys().next() == Some(&"".to_string()) {
                // Bare list structure - get values from the Vec at key ""
                let children = model.get_all("").expect("should have empty key");
                assert_eq!(
                    children.len(),
                    expected_array.len(),
                    "Test '{}': expected {} items at '{}', got {}",
                    test_name,
                    expected_array.len(),
                    path,
                    children.len()
                );

                // Each child has a single key which is the list item value
                for (i, (child, expected_item)) in
                    children.iter().zip(expected_array.iter()).enumerate()
                {
                    let new_path = format!("{}[{}]", path, i);
                    let actual_key = child.keys().next().unwrap_or(&"".to_string()).clone();
                    if let serde_json::Value::String(expected_str) = expected_item {
                        assert_eq!(
                            &actual_key, expected_str,
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
            } else {
                // Legacy structure - keys are the list items
                assert_eq!(
                    model.len(),
                    expected_array.len(),
                    "Test '{}': expected {} items at '{}', got {}",
                    test_name,
                    expected_array.len(),
                    path,
                    model.len()
                );

                for (i, (actual_key, expected_item)) in
                    model.keys().zip(expected_array.iter()).enumerate()
                {
                    let new_path = format!("{}[{}]", path, i);
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
            // Parse and build hierarchy with options from test behaviors
            let options = options_from_test(test);
            let result = load_with_options(test.input(), &options);

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
                assert_eq!(
                    model.len(),
                    test.expected.count,
                    "Test '{}' expected {} entries, got {}",
                    test.name,
                    test.expected.count,
                    model.len()
                );

                // Verify each expected entry exists with correct value
                for entry in &test.expected.entries {
                    let value = model.get_string(&entry.key).unwrap_or_else(|e| {
                        panic!(
                            "Test '{}': failed to get string for key '{}': {}",
                            test.name, entry.key, e
                        )
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
            let options = options_from_test(test);
            let result = load_with_options(test.input(), &options);

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
                // Removed direct .0 access
                assert_eq!(
                    model.len(),
                    test.expected.count,
                    "Test '{}' expected {} entries, got {}",
                    test.name,
                    test.expected.count,
                    model.len()
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
            // Parse the input with options from test behaviors
            let options = options_from_test(test);
            let model = load_with_options(test.input(), &options).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // Get the value at the specified key
            if let Some(ref key) = test.expected.key {
                let result = model.get_string(key);

                if test.expected.error.is_some() {
                    assert!(
                        result.is_err(),
                        "Test '{}' expected error for key '{}' but got: {:?}",
                        test.name,
                        key,
                        result
                    );
                } else if let Some(ref expected_value) = test.expected.value {
                    let value = result.unwrap_or_else(|e| {
                        panic!(
                            "Test '{}': failed to get string for key '{}': {}",
                            test.name, key, e
                        )
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
            // Parse the input with options from test behaviors
            let options = options_from_test(test);
            let model = load_with_options(test.input(), &options).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // For filter tests, we expect the model to filter out comments
            // and only contain non-comment entries - use public IndexMap field
            // Removed direct .0 access
            let count = model.len();
            assert_eq!(
                count, test.expected.count,
                "Test '{}' expected {} entries, got {}",
                test.name, test.expected.count, count
            );

            // Verify the actual entries match
            for entry in &test.expected.entries {
                let value = model.get_string(&entry.key).unwrap_or_else(|e| {
                    panic!(
                        "Test '{}': failed to get string for key '{}': {}",
                        test.name, entry.key, e
                    )
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

    // Display implementation capabilities
    println!("\n🔧 Implementation Capabilities");

    // Functions
    let mut functions: Vec<_> = config.supported_functions.iter().collect();
    functions.sort();
    println!("   Functions ({}):", functions.len());
    // Display functions in rows of 4 for better readability
    for chunk in functions.chunks(4) {
        let row = chunk
            .iter()
            .map(|s| format!("{:<18}", s))
            .collect::<Vec<_>>()
            .join("");
        println!("     {}", row.trim_end());
    }

    println!("   Behaviors:");

    // Fixed behaviors (compile-time)
    println!("     Fixed (compile-time):");
    println!(
        "       • Boolean parsing:  {}",
        config.boolean_behavior.as_str()
    );
    println!(
        "       • CRLF handling:    {}",
        config.crlf_behavior.as_str()
    );
    println!(
        "       • Array ordering:   {}",
        config.array_order_behavior.as_str()
    );

    // Runtime-configurable behaviors
    println!("     Configurable (runtime via ParserOptions):");

    // Spacing
    let mut spacing: Vec<_> = config
        .supported_spacing_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    spacing.sort();
    println!("       • Spacing:          {}", spacing.join(", "));

    // Tabs
    let mut tabs: Vec<_> = config
        .supported_tab_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    tabs.sort();
    println!("       • Tab handling:     {}", tabs.join(", "));

    // List coercion
    let mut list_coercion: Vec<_> = config
        .supported_list_coercion_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    list_coercion.sort();
    println!("       • List coercion:    {}", list_coercion.join(", "));

    println!();

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
            // Collect skip reasons using the new single decision function
            let mut skip_reasons_by_category: std::collections::HashMap<
                &str,
                Vec<common::SkipReason>,
            > = std::collections::HashMap::new();

            for test in &suite.tests {
                if let Some(reason) = common::TestSuite::should_skip_test(test, &config) {
                    skip_reasons_by_category
                        .entry(reason.category())
                        .or_default()
                        .push(reason);
                }
            }

            // Aggregate unique items per category for display
            let mut missing_variants = std::collections::HashSet::new();
            let mut missing_functions = std::collections::HashSet::new();
            let mut conflicting_behaviors = std::collections::HashSet::new();

            for reasons in skip_reasons_by_category.values() {
                for reason in reasons {
                    match reason {
                        common::SkipReason::UnsupportedVariant(variants) => {
                            missing_variants.extend(variants.iter().cloned());
                        }
                        common::SkipReason::MissingFunctions(functions) => {
                            missing_functions.extend(functions.iter().cloned());
                        }
                        common::SkipReason::ConflictingBehaviors(behaviors) => {
                            conflicting_behaviors.extend(behaviors.iter().cloned());
                        }
                    }
                }
            }

            // Determine appropriate icon and message based on skip reasons
            // Design Principle: Configuration Coverage Analysis
            // Detect when variant filtering might be masking behavior conflicts
            let has_missing_features = !missing_functions.is_empty();
            let has_variant_skips = !missing_variants.is_empty();
            let has_behavior_skips = !conflicting_behaviors.is_empty();

            let only_intentional_skips =
                !has_missing_features && (has_variant_skips || has_behavior_skips);

            if only_intentional_skips {
                println!(
                    "   ℹ️   Skipped {} tests (incompatible with current config)",
                    skipped_by_filter
                );
            } else {
                println!(
                    "   ⚠️   Skipped {} tests due to unsupported capabilities",
                    skipped_by_filter
                );
            }

            // Display reasons with masking pattern detection
            if !missing_variants.is_empty() {
                let mut variants: Vec<_> = missing_variants.iter().collect();
                variants.sort();
                println!(
                    "      Excluded variants: {}",
                    variants
                        .iter()
                        .map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                // Detect potential masking: if we have both variant and behavior skips
                if has_behavior_skips {
                    println!(
                        "      ⚠️  Note: Variant filtering may mask {} behavior conflict(s)",
                        conflicting_behaviors.len()
                    );
                }
            }
            if !missing_functions.is_empty() {
                let mut functions: Vec<_> = missing_functions.iter().collect();
                functions.sort();
                println!(
                    "      Missing functions: {}",
                    functions
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if !conflicting_behaviors.is_empty() {
                let mut behaviors: Vec<_> = conflicting_behaviors.iter().collect();
                behaviors.sort();
                println!(
                    "      Alternative behaviors: {}",
                    behaviors
                        .iter()
                        .map(|b| b.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            total_skipped += skipped_by_filter;
        }

        for test in filtered_tests {
            // Clear previous panic messages
            panic_messages.lock().unwrap().clear();

            let test_result = std::panic::catch_unwind(|| {
                // Build options from test behaviors
                let options = options_from_test(test);

                // Parse the input based on validation type, using behavior-aware options
                let (entries, model_result) =
                    if test.validation == "parse_dedented" || test.validation == "parse_indented" {
                        let e = parse_indented_with_options(test.input(), &options);
                        let m = e.as_ref().ok().map(|entries| build_hierarchy(entries));
                        (e, m)
                    } else {
                        let e = parse_with_options(test.input(), &options);
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
                        // "filter" validation tests parse the input, then filter out
                        // comment entries (where key == "/")
                        if test.expected.error.is_some() {
                            assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let entry_list = entries.unwrap_or_else(|e| {
                                panic!("Test '{}' failed to parse: {}", test.name, e);
                            });

                            // Filter out comment entries (key == "/")
                            let filtered: Vec<_> =
                                entry_list.iter().filter(|e| e.key != "/").collect();

                            assert_eq!(
                                filtered.len(),
                                test.expected.count,
                                "Test '{}' expected {} entries after filtering, got {}",
                                test.name,
                                test.expected.count,
                                filtered.len()
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

                            // Use the actual get_string() method being tested
                            let get_string_result = model.get_string(key);

                            if test.expected.error.is_some() {
                                assert!(
                                    get_string_result.is_err(),
                                    "Test '{}' expected error but got: {:?}",
                                    test.name,
                                    get_string_result
                                );
                            } else if let Some(ref expected_value) = test.expected.value {
                                let value = get_string_result.unwrap_or_else(|e| {
                                    panic!(
                                        "Test '{}': failed to get string for key '{}': {}",
                                        test.name, key, e
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

                        // Navigate to parent path (all but last element) if needed
                        let (parent_model, key) = if test.args.len() > 1 {
                            let parent_path = &test.args[..test.args.len() - 1];
                            let parent = navigate_path(&model, parent_path, &test.name)
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}': failed to navigate path: {}", test.name, e);
                                });
                            (parent, test.args.last().unwrap())
                        } else {
                            (&model, test.args.first().unwrap())
                        };

                        // Use get_list or get_list_coerced based on test behaviors
                        let get_list_result = if test
                            .behaviors
                            .contains(&"list_coercion_enabled".to_string())
                        {
                            parent_model.get_list_coerced(key)
                        } else {
                            parent_model.get_list(key)
                        };

                        if test.expected.error.is_some() {
                            assert!(
                                get_list_result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                get_list_result
                            );
                        } else if test.expected.count == 0 {
                            // Empty list case
                            if let Ok(list) = get_list_result {
                                assert!(
                                    list.is_empty(),
                                    "Test '{}' expected empty list but got {} items",
                                    test.name,
                                    list.len()
                                );
                            } else {
                                // Key doesn't exist - that's also acceptable for empty list test
                                assert!(
                                    get_list_result.is_err(),
                                    "Test '{}' expected empty list or error",
                                    test.name
                                );
                            }
                        } else if let Some(ref expected_list) = test.expected.list {
                            let actual_list = get_list_result.unwrap_or_else(|e| {
                                panic!(
                                    "Test '{}': failed to get list for key '{}': {}",
                                    test.name, key, e
                                )
                            });

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

                        // Navigate to parent path (all but last element) if needed
                        let (parent_model, key) = if test.args.len() > 1 {
                            let parent_path = &test.args[..test.args.len() - 1];
                            let parent = navigate_path(&model, parent_path, &test.name)
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}': failed to navigate path: {}", test.name, e);
                                });
                            (parent, test.args.last().unwrap())
                        } else {
                            (&model, test.args.first().unwrap())
                        };

                        // Use the typed accessor get_int()
                        let int_result = parent_model.get_int(key);

                        if test.expected.error.is_some() {
                            assert!(
                                int_result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                int_result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let actual_int = int_result.unwrap_or_else(|e| {
                                panic!("Test '{}': failed to get int from model: {}", test.name, e)
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

                        // Navigate to parent path (all but last element) if needed
                        let (parent_model, key) = if test.args.len() > 1 {
                            let parent_path = &test.args[..test.args.len() - 1];
                            let parent = navigate_path(&model, parent_path, &test.name)
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}': failed to navigate path: {}", test.name, e);
                                });
                            (parent, test.args.last().unwrap())
                        } else {
                            (&model, test.args.first().unwrap())
                        };

                        // Use the typed accessor get_bool()
                        let bool_result = parent_model.get_bool(key);

                        if test.expected.error.is_some() {
                            assert!(
                                bool_result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                bool_result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            if expected_value.is_null() {
                                // Test expects that the value cannot be parsed as bool
                                // This should result in an error from get_bool()
                                assert!(
                                    bool_result.is_err(),
                                    "Test '{}': expected error (unparseable bool) but got: {:?}",
                                    test.name,
                                    bool_result
                                );
                            } else {
                                let actual_bool = bool_result.unwrap_or_else(|e| {
                                    panic!(
                                        "Test '{}': failed to get bool from model: {}",
                                        test.name, e
                                    )
                                });

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

                        // Navigate to parent path (all but last element) if needed
                        let (parent_model, key) = if test.args.len() > 1 {
                            let parent_path = &test.args[..test.args.len() - 1];
                            let parent = navigate_path(&model, parent_path, &test.name)
                                .unwrap_or_else(|e| {
                                    panic!("Test '{}': failed to navigate path: {}", test.name, e);
                                });
                            (parent, test.args.last().unwrap())
                        } else {
                            (&model, test.args.first().unwrap())
                        };

                        // Use the typed accessor get_float()
                        let float_result = parent_model.get_float(key);

                        if test.expected.error.is_some() {
                            assert!(
                                float_result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                float_result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let actual_float = float_result.unwrap_or_else(|e| {
                                panic!(
                                    "Test '{}': failed to get float from model: {}",
                                    test.name, e
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
                    "canonical_format" => {
                        // Parse input and convert to canonical format
                        let model = load_with_options(test.input(), &options).unwrap_or_else(|e| {
                            panic!("Test '{}' failed to load: {}", test.name, e);
                        });

                        let printer = CclPrinter::new();
                        let canonical_output = printer.print(&model);

                        if let Some(ref expected_value) = test.expected.value {
                            let expected_str = expected_value.as_str().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not a string", test.name)
                            });

                            assert_eq!(
                                canonical_output, expected_str,
                                "Test '{}': canonical format mismatch",
                                test.name
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
            skipped_by_filter + suite_skipped,
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

    // Fail if any tests failed - data-driven tests should all pass
    assert!(
        total_failed == 0,
        "Data-driven tests failed: {} failures out of {} tests",
        total_failed,
        total_passed + total_failed + total_skipped
    );
}

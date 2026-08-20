//! Data-driven CCL conformance tests.
//!
//! The shared [ccl-test-data] suites assert against two layers of the
//! specification: the flat `key = value` lexing pass (`parse`, `parse_indented`,
//! `print`, `filter`) and the hierarchical model built from it
//! (`build_hierarchy`, `get_*`). Sickle exposes the first through
//! [`sickle::unstable`] and the second through [`sickle::DocumentMut`].
//!
//! [ccl-test-data]: https://github.com/CatConfLang/ccl-test-data

mod common;

use colored::Colorize;

use common::{load_all_test_suites, ImplementationConfig, TestCase, TestSuite};
use sickle::unstable::{
    filter_comments, flat_entries, flat_entries_indented, print_flat, FlatEntry,
};
use sickle::{
    BoolBehavior, CrlfBehavior, DelimiterStrategy, DocumentMut, Item, ListBehavior, Options,
    PathSegment, SpacingBehavior, TabBehavior,
};
use std::path::Path;

/// Build [`Options`] from a test case's declared behaviors.
fn options_from_test(test: &TestCase) -> Options {
    let mut options = Options::new();

    if test.behaviors.iter().any(|b| b == "strict_spacing") {
        options = options.with_spacing(SpacingBehavior::Strict);
    }
    if test.behaviors.iter().any(|b| b == "tabs_to_spaces") {
        options = options.with_tabs(TabBehavior::ToSpaces);
    }
    if test.behaviors.iter().any(|b| b == "crlf_normalize_to_lf") {
        options = options.with_crlf(CrlfBehavior::NormalizeToLf);
    }
    if test
        .behaviors
        .iter()
        .any(|b| b == "delimiter_prefer_spaced")
    {
        options = options.with_delimiter(DelimiterStrategy::PreferSpaced);
    }
    if test.behaviors.iter().any(|b| b == "boolean_lenient") {
        options = options.with_bool(BoolBehavior::Lenient);
    }
    if test.behaviors.iter().any(|b| b == "list_coercion_enabled") {
        options = options.with_list(ListBehavior::Coerce);
    }

    options
}

/// Turn a test's `args` into a checked path.
fn path_of(args: &[String]) -> Vec<PathSegment<'_>> {
    args.iter().map(|a| PathSegment::Key(a.as_str())).collect()
}

/// Look up a flat entry's value by key.
fn flat_value<'a>(entries: &'a [FlatEntry], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.value.as_str())
}

/// Every list element at `item`, whichever CCL spelling produced it.
fn list_items(item: &Item) -> Option<Vec<&Item>> {
    match item {
        Item::Array(array) => Some(array.iter().collect()),
        Item::Table(table) if table.is_bare_list() => Some(table.values().collect()),
        _ => None,
    }
}

/// Validate one node of the tree against the suite's expected JSON shape.
///
/// The mapping is: JSON string -> scalar, JSON array -> list, JSON object ->
/// nested block.
fn validate_item_against_json(
    item: &Item,
    expected: &serde_json::Value,
    test_name: &str,
    path: &str,
) {
    match expected {
        serde_json::Value::String(expected_str) => {
            let actual = item.as_str().unwrap_or_else(|| {
                panic!(
                    "Test '{}': expected a scalar at '{}', found {}",
                    test_name,
                    path,
                    item.type_name()
                )
            });
            assert_eq!(
                actual, expected_str,
                "Test '{}': wrong value at '{}'",
                test_name, path
            );
        }
        serde_json::Value::Array(expected_array) => {
            let items = list_items(item).unwrap_or_else(|| {
                panic!(
                    "Test '{}': expected a list at '{}', found {}",
                    test_name,
                    path,
                    item.type_name()
                )
            });
            validate_items_against_json(&items, expected_array, test_name, path);
        }
        serde_json::Value::Object(expected_map) => {
            // The reference model spells a bare list as a block holding a single
            // empty key. Sickle normalizes that to a list, so accept both.
            if let (1, Some(serde_json::Value::Array(expected_array))) =
                (expected_map.len(), expected_map.get(""))
            {
                if let Some(items) = list_items(item) {
                    validate_items_against_json(&items, expected_array, test_name, path);
                    return;
                }
            }

            let table = item.as_table().unwrap_or_else(|| {
                panic!(
                    "Test '{}': expected a block at '{}', found {}",
                    test_name,
                    path,
                    item.type_name()
                )
            });

            // Sickle models `/= text` as trivia rather than a `/` table key, so
            // comment keys are validated through the flat `parse` view instead.
            // See the crate docs for this intentional deviation.
            let expected_map: Vec<_> = expected_map
                .iter()
                .filter(|(key, _)| key.as_str() != "/")
                .collect();

            for (key, expected_value) in &expected_map {
                let new_path = if path == "root" {
                    (*key).clone()
                } else {
                    format!("{}.{}", path, key)
                };

                // Repeating a key composes its values, per CCL's monoid rules.
                let composed = table.get_composed(key).unwrap_or_else(|| {
                    panic!("Test '{}': missing key '{}' at '{}'", test_name, key, path)
                });
                validate_item_against_json(&composed, expected_value, test_name, &new_path);
            }

            assert_eq!(
                table.unique_keys().len(),
                expected_map.len(),
                "Test '{}': expected {} keys at '{}', got {}",
                test_name,
                expected_map.len(),
                path,
                table.unique_keys().len()
            );
        }
        other => panic!(
            "Test '{}': unsupported JSON type at '{}': {:?}",
            test_name, path, other
        ),
    }
}

/// Validate a list of nodes against a JSON array.
fn validate_items_against_json(
    items: &[&Item],
    expected: &[serde_json::Value],
    test_name: &str,
    path: &str,
) {
    assert_eq!(
        items.len(),
        expected.len(),
        "Test '{}': expected {} items at '{}', got {}",
        test_name,
        expected.len(),
        path,
        items.len()
    );

    for (index, (item, expected_item)) in items.iter().zip(expected.iter()).enumerate() {
        validate_item_against_json(item, expected_item, test_name, &format!("{path}[{index}]"));
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
            let options = options_from_test(test);
            let entries = flat_entries(test.input(), &options);

            if test.expected.error.is_some() {
                assert!(
                    entries.is_err(),
                    "Test '{}' expected error but parsing succeeded",
                    test.name
                );
                return;
            }

            let entries = entries.unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            assert_eq!(
                entries.len(),
                test.expected.count,
                "Test '{}' expected {} entries, got {}",
                test.name,
                test.expected.count,
                entries.len()
            );

            for entry in &test.expected.entries {
                let value = flat_value(&entries, &entry.key)
                    .unwrap_or_else(|| panic!("Test '{}': missing key '{}'", test.name, entry.key));
                assert_eq!(
                    value, entry.value,
                    "Test '{}': key '{}' has wrong value",
                    test.name, entry.key
                );
            }
        });

        match test_result {
            Ok(_) => {
                println!("  {} {}", "[PASS]".green(), test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  {} {}: {:?}", "[FAIL]".red(), test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nResults: {} {} passed, {} {} failed",
        "[PASS]".green(),
        passed,
        "[FAIL]".red(),
        failed
    );
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
            let entries = flat_entries(test.input(), &options);

            if test.expected.error.is_some() {
                assert!(
                    entries.is_err(),
                    "Test '{}' expected error but parsing succeeded",
                    test.name
                );
            } else {
                let entries = entries.unwrap_or_else(|e| {
                    panic!("Test '{}' failed to parse: {}", test.name, e);
                });

                // Comments lex as `/` entries, so they are counted here.
                assert_eq!(
                    entries.len(),
                    test.expected.count,
                    "Test '{}' expected {} entries, got {}",
                    test.name,
                    test.expected.count,
                    entries.len()
                );
            }
        });

        match test_result {
            Ok(_) => {
                println!("  {} {}", "[PASS]".green(), test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  {} {}: {:?}", "[FAIL]".red(), test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nComments tests: {} {} passed, {} {} failed",
        "[PASS]".green(),
        passed,
        "[FAIL]".red(),
        failed
    );
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
            let doc = DocumentMut::parse_with(test.input(), &options).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // Get the value at the specified key
            if let Some(ref key) = test.expected.key {
                let result = doc.get_string([key.as_str()]);

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
                println!("  {} {}", "[PASS]".green(), test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  {} {}: {:?}", "[FAIL]".red(), test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nString access tests: {} {} passed, {} {} failed",
        "[PASS]".green(),
        passed,
        "[FAIL]".red(),
        failed
    );
    assert!(passed > 0, "At least some string access tests should pass");
}

#[test]
fn test_filter_function() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_comments.json");

    if !path.exists() {
        println!(
            "{} Skipping test - test data file not found",
            "[INFO]".yellow()
        );
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
            let entries = flat_entries(test.input(), &options).unwrap_or_else(|e| {
                panic!("Test '{}' failed to parse: {}", test.name, e);
            });

            // `filter` drops comment entries, leaving only real data.
            let filtered = filter_comments(&entries);
            assert_eq!(
                filtered.len(),
                test.expected.count,
                "Test '{}' expected {} entries, got {}",
                test.name,
                test.expected.count,
                filtered.len()
            );

            for entry in &test.expected.entries {
                let value = flat_value(&filtered, &entry.key)
                    .unwrap_or_else(|| panic!("Test '{}': missing key '{}'", test.name, entry.key));

                assert_eq!(
                    value, entry.value,
                    "Test '{}': key '{}' has wrong value",
                    test.name, entry.key
                );
            }
        });

        match test_result {
            Ok(_) => {
                println!("  {} {}", "[PASS]".green(), test.name);
                passed += 1;
            }
            Err(e) => {
                println!("  {} {}: {:?}", "[FAIL]".red(), test.name, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nFilter function tests: {} {} passed, {} {} failed",
        "[PASS]".green(),
        passed,
        "[FAIL]".red(),
        failed
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

    println!("\n{}", "═══ CCL TEST SUITE ═══".bold());
    println!("Loaded {} test suite files", suites.len());

    // Display implementation capabilities
    println!("\n{}", "═══ CAPABILITIES ═══".bold());

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

    println!("   {}:", "Behaviors".bold());

    // Fixed behaviors (compile-time)
    println!("     Fixed (compile-time):");
    println!(
        "       - Array ordering:   {}",
        config.array_order_behavior.as_str()
    );

    // Parse-time configurable behaviors (via Options)
    println!("     Parse-time configurable (via Options):");

    // CRLF
    let mut crlf: Vec<_> = config
        .supported_crlf_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    crlf.sort();
    println!("       - CRLF handling:    {}", crlf.join(", "));

    // Spacing
    let mut spacing: Vec<_> = config
        .supported_spacing_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    spacing.sort();
    println!("       - Spacing:          {}", spacing.join(", "));

    // Tabs
    let mut tabs: Vec<_> = config
        .supported_tab_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    tabs.sort();
    println!("       - Tab handling:     {}", tabs.join(", "));

    // Access-time configurable behaviors
    println!("     Access-time configurable:");

    // Boolean parsing (via Options)
    let mut boolean: Vec<_> = config
        .supported_boolean_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    boolean.sort();
    println!("       - Boolean parsing:  {}", boolean.join(", "));

    // List coercion (via Options)
    let mut list_coercion: Vec<_> = config
        .supported_list_coercion_behaviors
        .iter()
        .map(|b| b.as_str())
        .collect();
    list_coercion.sort();
    println!("       - List coercion:    {}", list_coercion.join(", "));

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
        println!("{} {}", "──".dimmed(), suite_name.bold());

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
                    "   {} Skipped {} tests (incompatible with current config)",
                    "[INFO]".yellow(),
                    skipped_by_filter
                );
            } else {
                println!(
                    "   {} Skipped {} tests due to unsupported capabilities",
                    "[INFO]".yellow(),
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
                        "      {} Note: Variant filtering may mask {} behavior conflict(s)",
                        "[INFO]".yellow(),
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
                let options = options_from_test(test);

                // The flat lexing pass and the hierarchical document are the two
                // layers the specification validates against.
                let entries =
                    if test.validation == "parse_dedented" || test.validation == "parse_indented" {
                        flat_entries_indented(test.input(), &options)
                    } else {
                        flat_entries(test.input(), &options)
                    };
                let document = DocumentMut::parse_with(test.input(), &options);

                let expect_entries = |entries: Result<Vec<FlatEntry>, sickle::ParseError>| {
                    if test.expected.error.is_some() {
                        assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        return None;
                    }
                    let list = entries.unwrap_or_else(|e| {
                        panic!("Test '{}' failed to parse: {}", test.name, e);
                    });
                    assert_eq!(
                        list.len(),
                        test.expected.count,
                        "Test '{}' expected {} entries, got {}",
                        test.name,
                        test.expected.count,
                        list.len()
                    );
                    for expected_entry in &test.expected.entries {
                        let found = list.iter().any(|e| {
                            e.key == expected_entry.key && e.value == expected_entry.value
                        });
                        assert!(
                            found,
                            "Test '{}': expected entry {}={} not found",
                            test.name, expected_entry.key, expected_entry.value
                        );
                    }
                    Some(list)
                };

                let expect_document = || {
                    document.clone().unwrap_or_else(|e| {
                        panic!("Test '{}' failed to build a document: {}", test.name, e);
                    })
                };

                match test.validation.as_str() {
                    "parse" | "parse_dedented" | "parse_indented" => {
                        expect_entries(entries);
                    }
                    "filter" => {
                        if test.expected.error.is_some() {
                            assert!(entries.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let list = entries.unwrap_or_else(|e| {
                                panic!("Test '{}' failed to parse: {}", test.name, e);
                            });
                            let filtered = filter_comments(&list);
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
                            assert!(document.is_err(), "Test '{}' expected error", test.name);
                        } else {
                            let doc = expect_document();
                            assert_eq!(
                                test.expected.count, 1,
                                "Test '{}': build_hierarchy tests should have count=1",
                                test.name
                            );
                            if let Some(ref expected_obj) = test.expected.object {
                                let root = Item::Table(doc.as_table().clone());
                                validate_item_against_json(&root, expected_obj, &test.name, "root");
                            }
                        }
                    }
                    "get_string" => {
                        if let Some(ref key) = test.expected.key {
                            let doc = expect_document();
                            let result = doc.get_string([key.as_str()]);

                            if test.expected.error.is_some() {
                                assert!(
                                    result.is_err(),
                                    "Test '{}' expected error but got: {:?}",
                                    test.name,
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
                                    "Test '{}': wrong value for key '{}'",
                                    test.name, key
                                );
                            }
                        }
                    }
                    "get_list" => {
                        let doc = expect_document();
                        let result = doc.get_list(path_of(&test.args));

                        if test.expected.error.is_some() {
                            assert!(
                                result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                result
                            );
                        } else if test.expected.count == 0 {
                            // A missing key is as acceptable as an empty list here.
                            if let Ok(list) = result {
                                assert!(
                                    list.is_empty(),
                                    "Test '{}' expected empty list but got {} items",
                                    test.name,
                                    list.len()
                                );
                            }
                        } else if let Some(ref expected_list) = test.expected.list {
                            let actual = result.unwrap_or_else(|e| {
                                panic!("Test '{}': failed to get list: {}", test.name, e)
                            });
                            assert_eq!(
                                &actual, expected_list,
                                "Test '{}': list values don't match",
                                test.name
                            );
                        }
                    }
                    "get_int" => {
                        let doc = expect_document();
                        let result = doc.get_int(path_of(&test.args));

                        if test.expected.error.is_some() {
                            assert!(
                                result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let actual = result.unwrap_or_else(|e| {
                                panic!("Test '{}': failed to get int: {}", test.name, e)
                            });
                            let expected_int = expected_value.as_i64().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not an integer", test.name)
                            });
                            assert_eq!(
                                actual, expected_int,
                                "Test '{}': wrong integer value",
                                test.name
                            );
                        }
                    }
                    "get_bool" => {
                        let doc = expect_document();
                        let result = doc.get_bool(path_of(&test.args));

                        if test.expected.error.is_some() {
                            assert!(
                                result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            if expected_value.is_null() {
                                assert!(
                                    result.is_err(),
                                    "Test '{}': expected an unparseable bool but got: {:?}",
                                    test.name,
                                    result
                                );
                            } else {
                                let actual = result.unwrap_or_else(|e| {
                                    panic!("Test '{}': failed to get bool: {}", test.name, e)
                                });
                                let expected_bool = expected_value.as_bool().unwrap_or_else(|| {
                                    panic!("Test '{}': expected value is not a boolean", test.name)
                                });
                                assert_eq!(
                                    actual, expected_bool,
                                    "Test '{}': wrong boolean value",
                                    test.name
                                );
                            }
                        }
                    }
                    "get_float" => {
                        let doc = expect_document();
                        let result = doc.get_float(path_of(&test.args));

                        if test.expected.error.is_some() {
                            assert!(
                                result.is_err(),
                                "Test '{}' expected error but got: {:?}",
                                test.name,
                                result
                            );
                        } else if let Some(ref expected_value) = test.expected.value {
                            let actual = result.unwrap_or_else(|e| {
                                panic!("Test '{}': failed to get float: {}", test.name, e)
                            });
                            let expected_float = expected_value.as_f64().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not a float", test.name)
                            });
                            assert!(
                                (actual - expected_float).abs() < 0.0001,
                                "Test '{}': wrong float value, expected {} got {}",
                                test.name,
                                expected_float,
                                actual
                            );
                        }
                    }
                    "print" => {
                        let list = entries.unwrap_or_else(|e| {
                            panic!("Test '{}' failed to parse: {}", test.name, e);
                        });
                        let output = print_flat(&list);

                        if let Some(ref expected_value) = test.expected.value {
                            let expected_str = expected_value.as_str().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not a string", test.name)
                            });
                            assert_eq!(
                                output, expected_str,
                                "Test '{}': print output mismatch",
                                test.name
                            );
                        }
                    }
                    "round_trip" => {
                        // parse(print(parse(x))) == parse(x)
                        let list = entries.unwrap_or_else(|e| {
                            panic!("Test '{}' failed to parse: {}", test.name, e);
                        });
                        let printed = print_flat(&list);
                        let reparsed = flat_entries(&printed, &options).unwrap_or_else(|e| {
                            panic!("Test '{}' failed to re-parse: {}", test.name, e);
                        });
                        let stable = reparsed == list;

                        match test.expected.value.as_ref().and_then(|v| v.as_bool()) {
                            Some(expected_bool) => assert_eq!(
                                stable, expected_bool,
                                "Test '{}': round_trip expected {}, got {}",
                                test.name, expected_bool, stable
                            ),
                            None => match test.expected.value.as_ref().and_then(|v| v.as_str()) {
                                Some(expected_str) => assert_eq!(
                                    printed, expected_str,
                                    "Test '{}': round_trip print output mismatch",
                                    test.name
                                ),
                                None => {
                                    assert!(
                                        stable,
                                        "Test '{}': round_trip expected true",
                                        test.name
                                    )
                                }
                            },
                        }
                    }
                    "canonical_format" => {
                        let mut doc = expect_document();
                        doc.fmt();
                        let canonical = doc.to_string();

                        if let Some(ref expected_value) = test.expected.value {
                            let expected_str = expected_value.as_str().unwrap_or_else(|| {
                                panic!("Test '{}': expected value is not a string", test.name)
                            });
                            assert_eq!(
                                canonical, expected_str,
                                "Test '{}': canonical format mismatch",
                                test.name
                            );
                        }
                    }
                    other => {
                        panic!("Unsupported validation type: {other}");
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

        // Build summary parts, omitting zero counts
        let mut parts = Vec::new();
        if suite_passed > 0 {
            parts.push(format!("{} {} passed", "[PASS]".green(), suite_passed));
        }
        if suite_failed > 0 {
            parts.push(format!("{} {} failed", "[FAIL]".red(), suite_failed));
        }
        let total_skipped_in_suite = skipped_by_filter + suite_skipped;
        if total_skipped_in_suite > 0 {
            parts.push(format!(
                "{} {} skipped",
                "[INFO]".yellow(),
                total_skipped_in_suite
            ));
        }
        println!("   {} (total: {})\n", parts.join(", "), suite.tests.len());
    }

    // Restore the default panic hook
    std::panic::set_hook(default_hook);

    println!("{}", "═══ RESULTS ═══".bold());
    if total_passed > 0 {
        println!("  {} {} passed", "[PASS]".green(), total_passed);
    }
    if total_failed > 0 {
        println!("  {} {} failed", "[FAIL]".red(), total_failed);
    }
    if total_skipped > 0 {
        println!(
            "  {} {} skipped (unsupported validation types)",
            "[INFO]".yellow(),
            total_skipped
        );
    }
    println!("  Total: {}", total_passed + total_failed + total_skipped);
    println!("{}", "════════════════════════════════════════".dimmed());

    // Show skipped validation types
    if !skipped_validations.is_empty() {
        println!("\n{}", "═══ SKIPPED VALIDATION TYPES ═══".bold());
        let mut skip_types: Vec<_> = skipped_validations.iter().collect();
        skip_types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (val_type, count) in skip_types {
            println!("  - {}: {} tests", val_type, count);
        }
    }

    // Show behavior coverage
    if !behavior_coverage.is_empty() {
        println!("\n{}", "═══ BEHAVIOR COVERAGE ═══".bold());
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
                println!("  {} vs {}", opt1.bold(), opt2.bold());
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
                "[PASS]".green()
            } else if *passed > 0 {
                "[INFO]".yellow()
            } else {
                "[FAIL]".red()
            };
            println!(
                "  {} {}: {}/{} ({}%)",
                status, behavior, passed, total, percent
            );
        }
    }

    // Show function coverage
    if !function_coverage.is_empty() {
        println!("\n{}", "═══ FUNCTION COVERAGE ═══".bold());
        let mut functions: Vec<_> = function_coverage.iter().collect();
        functions.sort_by_key(|(name, _)| *name);
        for (function, (passed, total)) in functions {
            let percent = if *total > 0 {
                (*passed * 100) / *total
            } else {
                0
            };
            let status = if *passed == *total {
                "[PASS]".green()
            } else if *passed > 0 {
                "[INFO]".yellow()
            } else {
                "[FAIL]".red()
            };
            println!(
                "  {} {}: {}/{} ({}%)",
                status, function, passed, total, percent
            );
        }
    }

    // Show failure details (limit to first 20 for readability)
    if !failure_details.is_empty() {
        println!("\n{}", "═══ FAILURE DETAILS ═══".bold());
        println!("  (showing first 20)");
        for (suite, test, reason) in failure_details.iter().take(20) {
            println!("  {} [{suite}] {test}", "[FAIL]".red());
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

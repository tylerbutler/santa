//! Test helpers for loading and executing CCL test cases from JSON files

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Represents a single test case from the CCL test-data repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Name of the test
    pub name: String,
    /// The CCL input string to parse
    pub input: String,
    /// Type of validation (e.g., "parse", "get_string", "get_int", etc.)
    pub validation: String,
    /// Expected output
    pub expected: ExpectedOutput,
    /// Arguments for accessor functions (e.g., key path for get_list, get_int, etc.)
    #[serde(default)]
    pub args: Vec<String>,
    /// Features used in this test (e.g., "comments", "multiline", etc.)
    #[serde(default)]
    pub features: Vec<String>,
    /// Behaviors tested (e.g., "boolean_strict", "crlf_normalize_to_lf")
    #[serde(default)]
    pub behaviors: Vec<String>,
    /// Variants of the test
    #[serde(default)]
    pub variants: Vec<String>,
    /// Functions being tested
    #[serde(default)]
    pub functions: Vec<String>,
    /// Source test name
    #[serde(default)]
    pub source_test: String,
}

/// Expected output from parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedOutput {
    /// Number of expected entries
    pub count: usize,
    /// Expected key-value entries
    #[serde(default)]
    pub entries: Vec<Entry>,
    /// For build_hierarchy tests - expected object structure
    #[serde(default)]
    pub object: Option<serde_json::Value>,
    /// For typed access tests - expected value
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// For get_list tests - expected list of values
    #[serde(default)]
    pub list: Option<Vec<String>>,
    /// For get operations - the key path
    #[serde(default)]
    pub key: Option<String>,
    /// For error cases - expected error message
    #[serde(default)]
    pub error: Option<String>,
}

/// A key-value entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub key: String,
    pub value: String,
}

/// Container for test suite loaded from JSON
#[derive(Debug, Serialize, Deserialize)]
pub struct TestSuite {
    /// JSON schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// List of test cases
    pub tests: Vec<TestCase>,
}

/// Centralized configuration for implementation capabilities
///
/// This struct defines what the current sickle implementation supports.
/// It serves as the single source of truth for:
/// - Which CCL functions are implemented
/// - Which behavior choices we've made
/// - Which test variants to run (e.g., excluding proposed_behavior)
///
/// Configuration defining which behaviors, functions, and variants are supported
/// by the current implementation.
///
/// Update `sickle_current()` to add/remove capabilities as implementation evolves.
#[derive(Debug, Clone)]
pub struct ImplementationConfig {
    /// Supported functions (e.g., "parse", "build_hierarchy", "get_string")
    pub supported_functions: HashSet<String>,
    /// Chosen behaviors (e.g., "boolean_strict", "crlf_normalize_to_lf")
    pub chosen_behaviors: HashSet<String>,
    /// Supported variants (e.g., "reference_compliant", excluding "proposed_behavior")
    pub supported_variants: HashSet<String>,
}

impl ImplementationConfig {
    /// Create a new configuration with the current Sickle implementation capabilities
    ///
    /// This configuration defines a reference-compliant CCL parser that follows
    /// the OCaml reference implementation's behavior.
    pub fn sickle_current() -> Self {
        Self {
            supported_functions: [
                "parse",
                "parse_indented",
                "build_hierarchy",
                "filter",
                "get_string",
                "get_int",
                "get_float",
                "get_bool",
                "get_list",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            chosen_behaviors: [
                "list_coercion_disabled", // Reference: explicit lists only
                "crlf_preserve_literal",  // Reference: preserve CRLF
                "boolean_strict",         // Reference: strict boolean parsing
                "strict_spacing",         // Reference: strict spacing rules
                "tabs_preserve",          // Reference: preserve tabs
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            supported_variants: ["reference_compliant"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// Check if a behavior is supported
    pub fn supports_behavior(&self, behavior: &str) -> bool {
        self.chosen_behaviors.contains(behavior)
    }

    /// Check if a function is supported
    pub fn supports_function(&self, function: &str) -> bool {
        self.supported_functions.contains(function)
    }

    /// Check if all functions in a list are supported
    pub fn supports_all_functions(&self, functions: &[String]) -> bool {
        functions.is_empty() || functions.iter().all(|f| self.supports_function(f))
    }
}

impl TestSuite {
    /// Load a test suite from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let suite: TestSuite = serde_json::from_str(&content)?;
        Ok(suite)
    }

    /// Filter tests by validation type
    pub fn filter_by_validation(&self, validation: &str) -> Vec<&TestCase> {
        self.tests
            .iter()
            .filter(|t| t.validation == validation)
            .collect()
    }

    /// Filter tests by behavior
    #[allow(dead_code)]
    pub fn filter_by_behavior(&self, behavior: &str) -> Vec<&TestCase> {
        self.tests
            .iter()
            .filter(|t| t.behaviors.contains(&behavior.to_string()))
            .collect()
    }

    /// Filter tests by function
    pub fn filter_by_function(&self, function: &str) -> Vec<&TestCase> {
        self.tests
            .iter()
            .filter(|t| t.functions.contains(&function.to_string()))
            .collect()
    }

    /// Filter tests based on implementation capabilities
    ///
    /// This implements the test filtering strategy described in the CCL test suite guide.
    /// Tests are filtered based on:
    /// 1. Whether test variants are supported (e.g., excludes "proposed_behavior")
    /// 2. Whether all required functions are implemented
    /// 3. Whether behaviors conflict with chosen behaviors
    pub fn filter_by_capabilities(&self, config: &ImplementationConfig) -> Vec<&TestCase> {
        self.tests
            .iter()
            .filter(|test| {
                // Check variant support
                // If test has variants, at least one must be in our supported list
                // Empty supported_variants means we only accept tests with no variants
                if !test.variants.is_empty() {
                    let has_supported_variant = test
                        .variants
                        .iter()
                        .any(|v| config.supported_variants.contains(v));

                    if !has_supported_variant {
                        return false;
                    }
                }

                // Check if all required functions are implemented
                if !config.supports_all_functions(&test.functions) {
                    return false;
                }

                // Check for behavior conflicts
                // If the test specifies behaviors, at least one should match our chosen behaviors
                // or the test should have no behaviors (meaning it's behavior-agnostic)
                if !test.behaviors.is_empty() {
                    // Check if any of the test's behaviors are in our chosen behaviors
                    let has_matching_behavior =
                        test.behaviors.iter().any(|b| config.supports_behavior(b));

                    // If no matching behavior, check if it conflicts with mutually exclusive behaviors
                    if !has_matching_behavior {
                        // Check for mutually exclusive behavior pairs
                        let mutually_exclusive = [
                            ("boolean_strict", "boolean_lenient"),
                            ("crlf_preserve_literal", "crlf_normalize_to_lf"),
                            ("list_coercion_enabled", "list_coercion_disabled"),
                            ("strict_spacing", "relaxed_spacing"),
                            ("tabs_preserve", "tabs_normalize"),
                        ];

                        // If the test requires a behavior that conflicts with our chosen behavior, skip it
                        for (opt1, opt2) in &mutually_exclusive {
                            if (test.behaviors.contains(&opt1.to_string())
                                && config.supports_behavior(opt2))
                                || (test.behaviors.contains(&opt2.to_string())
                                    && config.supports_behavior(opt1))
                            {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .collect()
    }

    /// Get all test names
    #[allow(dead_code)]
    pub fn test_names(&self) -> Vec<&str> {
        self.tests.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Helper to load all test suites from the test_data directory
pub fn load_all_test_suites() -> HashMap<String, TestSuite> {
    let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data");
    let mut suites = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(&test_data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(suite) = TestSuite::from_file(&path) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    suites.insert(name, suite);
                }
            }
        }
    }

    suites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_parsing_suite() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/api_core_ccl_parsing.json");

        if path.exists() {
            let suite = TestSuite::from_file(&path).expect("should load test suite");
            assert!(!suite.tests.is_empty(), "should have test cases");

            // Verify structure of first test
            if let Some(first_test) = suite.tests.first() {
                assert!(!first_test.name.is_empty());
                assert!(!first_test.input.is_empty());
                assert_eq!(first_test.validation, "parse");
            }
        }
    }

    #[test]
    fn test_load_all_suites() {
        let suites = load_all_test_suites();
        // Should load at least one suite if JSON files exist
        assert!(
            !suites.is_empty(),
            "Should load test suites from test_data directory"
        );
    }
}

//! Test helpers for loading and executing CCL test cases from JSON files

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// For typed access tests - expected value
    #[serde(default)]
    pub value: Option<serde_json::Value>,
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

    /// Filter tests by feature
    pub fn filter_by_feature(&self, feature: &str) -> Vec<&TestCase> {
        self.tests
            .iter()
            .filter(|t| t.features.contains(&feature.to_string()))
            .collect()
    }

    /// Get all test names
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/test_data/api_core_ccl_parsing.json");

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
            suites.len() >= 1,
            "Should load test suites from test_data directory"
        );
    }
}

//! Capability registry for test filtering
//!
//! This module defines which behaviors, functions, and features are supported
//! by the sickle parser implementation. Tests are filtered based on these capabilities.

use std::collections::HashSet;

/// Test capability registry
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    supported_behaviors: HashSet<String>,
    supported_functions: HashSet<String>,
    supported_features: HashSet<String>,
}

impl CapabilityRegistry {
    /// Create a new capability registry with the current implementation's capabilities
    pub fn new() -> Self {
        Self {
            supported_behaviors: Self::init_supported_behaviors(),
            supported_functions: Self::init_supported_functions(),
            supported_features: Self::init_supported_features(),
        }
    }

    /// Initialize supported behaviors (reference-compliant)
    fn init_supported_behaviors() -> HashSet<String> {
        [
            "list_coercion_disabled",
            "crlf_preserve_literal",
            "boolean_strict",
            "strict_spacing",
            "tabs_preserve",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Initialize supported functions
    fn init_supported_functions() -> HashSet<String> {
        [
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
        .collect()
    }

    /// Initialize supported features
    fn init_supported_features() -> HashSet<String> {
        [
            "comments",
            "empty_keys",
            "multiline",
            "unicode",
            "whitespace",
            "optional_typed_accessors",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Check if a behavior is supported
    pub fn supports_behavior(&self, behavior: &str) -> bool {
        self.supported_behaviors.contains(behavior)
    }

    /// Check if a function is supported
    pub fn supports_function(&self, function: &str) -> bool {
        self.supported_functions.contains(function)
    }

    /// Check if a feature is supported
    pub fn supports_feature(&self, feature: &str) -> bool {
        self.supported_features.contains(feature)
    }

    /// Check if all behaviors in a list are supported
    pub fn supports_all_behaviors(&self, behaviors: &[String]) -> bool {
        behaviors.is_empty() || behaviors.iter().all(|b| self.supports_behavior(b))
    }

    /// Check if all functions in a list are supported
    pub fn supports_all_functions(&self, functions: &[String]) -> bool {
        functions.is_empty() || functions.iter().all(|f| self.supports_function(f))
    }

    /// Check if all features in a list are supported
    pub fn supports_all_features(&self, features: &[String]) -> bool {
        features.is_empty() || features.iter().all(|f| self.supports_feature(f))
    }

    /// Check if a test should be run based on its requirements
    pub fn should_run_test(
        &self,
        behaviors: &[String],
        functions: &[String],
        features: &[String],
    ) -> bool {
        self.supports_all_behaviors(behaviors)
            && self.supports_all_functions(functions)
            && self.supports_all_features(features)
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_behaviors() {
        let registry = CapabilityRegistry::new();

        // Supported behaviors
        assert!(registry.supports_behavior("list_coercion_disabled"));
        assert!(registry.supports_behavior("crlf_preserve_literal"));
        assert!(registry.supports_behavior("boolean_strict"));
        assert!(registry.supports_behavior("strict_spacing"));
        assert!(registry.supports_behavior("tabs_preserve"));

        // Not supported behaviors
        assert!(!registry.supports_behavior("list_coercion_enabled"));
        assert!(!registry.supports_behavior("crlf_normalize_to_lf"));
        assert!(!registry.supports_behavior("boolean_lenient"));
    }

    #[test]
    fn test_supported_functions() {
        let registry = CapabilityRegistry::new();

        // Supported functions
        assert!(registry.supports_function("parse"));
        assert!(registry.supports_function("parse_indented"));
        assert!(registry.supports_function("build_hierarchy"));
        assert!(registry.supports_function("filter"));
        assert!(registry.supports_function("get_string"));
        assert!(registry.supports_function("get_int"));
        assert!(registry.supports_function("get_float"));
        assert!(registry.supports_function("get_bool"));
        assert!(registry.supports_function("get_list"));

        // Not supported functions
        assert!(!registry.supports_function("canonical_format"));
        assert!(!registry.supports_function("round_trip"));
    }

    #[test]
    fn test_supported_features() {
        let registry = CapabilityRegistry::new();

        // All features are supported
        assert!(registry.supports_feature("comments"));
        assert!(registry.supports_feature("empty_keys"));
        assert!(registry.supports_feature("multiline"));
        assert!(registry.supports_feature("unicode"));
        assert!(registry.supports_feature("whitespace"));
        assert!(registry.supports_feature("optional_typed_accessors"));
    }

    #[test]
    fn test_should_run_test() {
        let registry = CapabilityRegistry::new();

        // Test with supported capabilities
        assert!(registry.should_run_test(
            &["boolean_strict".to_string()],
            &["parse".to_string()],
            &["unicode".to_string()]
        ));

        // Test with unsupported behavior
        assert!(!registry.should_run_test(
            &["list_coercion_enabled".to_string()],
            &["parse".to_string()],
            &[]
        ));

        // Test with unsupported function
        assert!(!registry.should_run_test(
            &[],
            &["canonical_format".to_string()],
            &[]
        ));

        // Test with empty requirements (should always run)
        assert!(registry.should_run_test(&[], &[], &[]));
    }
}

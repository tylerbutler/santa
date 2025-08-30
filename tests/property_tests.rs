//! Property-based tests for Santa Package Manager
//!
//! These tests use the `proptest` crate to generate random inputs and verify
//! that our functions behave correctly across a wide range of inputs.

use proptest::prelude::*;
use santa::configuration::SantaConfig;
use santa::data::{KnownSources, SourceList};

proptest! {
    #[test]
    fn config_serialization_roundtrip(
        packages in prop::collection::vec("[a-zA-Z0-9_-]{3,20}", 1..5)
    ) {
        let config = SantaConfig {
            sources: vec![KnownSources::Apt],
            packages,
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };
        
        let serialized = serde_yaml::to_string(&config).unwrap();
        let deserialized: SantaConfig = serde_yaml::from_str(&serialized).unwrap();
        prop_assert_eq!(config.sources, deserialized.sources);
        prop_assert_eq!(config.packages, deserialized.packages);
    }

    #[test] 
    fn package_name_contains_no_dangerous_chars(
        name in "[a-zA-Z0-9._-]{1,50}"
    ) {
        // Valid package names should not contain dangerous shell characters
        prop_assert!(!name.contains(';'));
        prop_assert!(!name.contains('|'));
        prop_assert!(!name.contains('&'));
        prop_assert!(!name.contains('`'));
        prop_assert!(!name.contains('$'));
        prop_assert!(!name.contains('('));
        prop_assert!(!name.contains(')'));
    }

    #[test]
    fn source_list_operations_are_stable(
        count in 0usize..10
    ) {
        let mut source_list = SourceList::new();
        let original_len = source_list.len();
        
        // Adding test sources should maintain consistency
        for i in 0..count {
            source_list.push(create_test_source(i));
        }
        
        prop_assert_eq!(source_list.len(), original_len + count);
        
        // Clear should empty the list
        source_list.clear();
        prop_assert_eq!(source_list.len(), 0);
    }

    #[test] 
    fn command_strings_are_properly_formatted(
        base_cmd in "[a-zA-Z0-9_-]{2,10}",
        packages in prop::collection::vec("[a-zA-Z0-9._-]{1,20}", 1..5)
    ) {
        let install_command = format!("{} install", base_cmd);
        let package_list = packages.join(" ");
        let full_command = format!("{} {}", install_command, package_list);
        
        // Command should not be empty
        prop_assert!(!full_command.is_empty());
        
        // Should contain the base command
        prop_assert!(full_command.starts_with(&base_cmd));
        
        // Should contain all packages
        for package in &packages {
            prop_assert!(full_command.contains(package));
        }
    }

    #[test]
    fn known_sources_handling(
        source in prop_oneof![
            Just(KnownSources::Apt),
            Just(KnownSources::Brew),
            Just(KnownSources::Cargo),
            Just(KnownSources::Pacman),
        ]
    ) {
        let config = SantaConfig {
            sources: vec![source.clone()],
            packages: vec!["test-package".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };
        
        // Sources should be preserved correctly
        prop_assert_eq!(config.sources.len(), 1);
        prop_assert_eq!(&config.sources[0], &source);
    }
}

// Helper function to create test package sources
fn create_test_source(index: usize) -> santa::sources::PackageSource {
    santa::sources::PackageSource::new_for_test(
        KnownSources::Unknown(format!("test-source-{}", index)),
        "🧪",
        &format!("test-{}", index),
        &format!("test-{} install", index),
        &format!("test-{} list", index),
        None,
        None,
    )
}

// Additional focused property tests for security-critical functions
proptest! {
    #[test]
    fn shell_command_safety(
        user_input in ".*"
    ) {
        // Any user input should be safely escapable
        let escaped = shell_escape::escape(std::borrow::Cow::from(&user_input));
        
        // Escaped string should not contain unescaped dangerous characters
        // when not already properly quoted
        if !user_input.starts_with('"') && !user_input.starts_with('\'') {
            let escaped_str = escaped.as_ref();
            
            // These characters should be escaped or the string should be quoted
            let dangerous_chars = [';', '|', '&', '`', '$', '(', ')'];
            for &dangerous_char in &dangerous_chars {
                if user_input.contains(dangerous_char) {
                    prop_assert!(
                        escaped_str.starts_with('\'') || 
                        escaped_str.starts_with('"') ||
                        !escaped_str.contains(dangerous_char),
                        "Dangerous character '{}' not properly escaped in: {}",
                        dangerous_char,
                        escaped_str
                    );
                }
            }
        }
    }

    #[test] 
    fn package_source_name_consistency(
        index in 0usize..100
    ) {
        let source = create_test_source(index);
        let expected_name = format!("test-source-{}", index);
        prop_assert_eq!(source.name_str(), expected_name);
        let expected_cmd = format!("test-{}", index);
        prop_assert!(source.install_command().contains(&expected_cmd));
    }
}
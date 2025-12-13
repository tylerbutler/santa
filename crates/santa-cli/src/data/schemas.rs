// Schema-based data structures for Santa Package Manager
// These structs match the YAML schemas defined in /data/*.yaml files

use crate::data::{Platform, OS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package definition matching package_schema.yaml
/// Supports both simple array format and complex object format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageDefinition {
    /// Simple array format: just a list of source names
    Simple(Vec<String>),
    /// Complex object format: with metadata and source-specific configs
    Complex(ComplexPackageDefinition),
}

impl PackageDefinition {
    /// Get all sources where this package is available
    pub fn get_sources(&self) -> Vec<&str> {
        match self {
            PackageDefinition::Simple(sources) => sources.iter().map(|s| s.as_str()).collect(),
            PackageDefinition::Complex(complex) => complex.get_sources(),
        }
    }

    /// Get source-specific configuration for a source
    pub fn get_source_config(&self, source: &str) -> Option<&SourceSpecificConfig> {
        match self {
            PackageDefinition::Simple(_) => None,
            PackageDefinition::Complex(complex) => complex.get_source_config(source),
        }
    }

    /// Check if package is available in a specific source
    pub fn is_available_in(&self, source: &str) -> bool {
        self.get_sources().contains(&source)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[non_exhaustive]
pub struct ComplexPackageDefinition {
    /// List of sources where package is available with same name as key
    #[serde(rename = "_sources", skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,

    /// Platforms where this package is available
    #[serde(rename = "_platforms", skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,

    /// Alternative names for search and discovery
    #[serde(rename = "_aliases", skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,

    /// Source-specific configurations (flatten other fields)
    #[serde(flatten)]
    pub source_configs: HashMap<String, SourceSpecificConfig>,
}

/// Source-specific configuration for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceSpecificConfig {
    /// Simple name override
    Name(String),
    /// Complex configuration with hooks and modifications
    Complex(SourceConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Override package name for this source
    pub name: Option<String>,
    /// Command to run before installation
    pub pre: Option<String>,
    /// Command to run after successful installation  
    pub post: Option<String>,
    /// String to prepend to package name during installation
    pub prefix: Option<String>,
    /// String to append to the install command
    pub install_suffix: Option<String>,
}

/// Sources configuration matching sources_schema.yaml
pub type SourcesDefinition = HashMap<String, SourceDefinition>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDefinition {
    /// Emoji icon to represent this source
    pub emoji: String,
    /// Command template to install packages
    pub install: String,
    /// Command to list installed packages from this source
    pub check: String,
    /// String to prepend to package names (optional)
    pub prefix: Option<String>,
    /// Platform-specific command overrides
    #[serde(rename = "_overrides", skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, PlatformOverride>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformOverride {
    pub install: Option<String>,
    pub check: Option<String>,
}

/// Configuration matching config_schema.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDefinition {
    /// List of package sources to use (in priority order)
    pub sources: Vec<String>,
    /// List of packages to install/manage
    pub packages: Vec<String>,
    /// Advanced configuration options
    #[serde(rename = "_settings", skip_serializing_if = "Option::is_none")]
    pub settings: Option<ConfigSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSettings {
    /// Automatically update packages
    #[serde(default)]
    pub auto_update: bool,
    /// Maximum parallel package installations
    #[serde(default = "default_parallel_installs")]
    pub parallel_installs: u8,
    /// Ask for confirmation before installing packages
    #[serde(default = "default_true")]
    pub confirm_before_install: bool,
}

fn default_parallel_installs() -> u8 {
    3
}
fn default_true() -> bool {
    true
}

impl ComplexPackageDefinition {
    /// Create a new ComplexPackageDefinition with the given sources
    pub fn with_sources(sources: Vec<String>) -> Self {
        Self {
            sources: Some(sources),
            ..Default::default()
        }
    }

    /// Get all sources where this package is available
    pub fn get_sources(&self) -> Vec<&str> {
        let mut all_sources = Vec::new();

        // Add sources from _sources array
        if let Some(sources) = &self.sources {
            all_sources.extend(sources.iter().map(|s| s.as_str()));
        }

        // Add sources from explicit configurations
        all_sources.extend(self.source_configs.keys().map(|s| s.as_str()));

        all_sources
    }

    /// Get source-specific configuration for a source
    pub fn get_source_config(&self, source: &str) -> Option<&SourceSpecificConfig> {
        self.source_configs.get(source)
    }

    /// Check if package is available in a specific source
    pub fn is_available_in(&self, source: &str) -> bool {
        self.get_sources().contains(&source)
    }
}

impl SourceDefinition {
    /// Get the appropriate command for the current platform
    pub fn get_install_command(&self, platform: &Platform) -> &str {
        if let Some(overrides) = &self.overrides {
            let platform_key = match platform.os {
                OS::Windows => "windows",
                OS::Linux => "linux",
                OS::Macos => "macos",
                _ => return &self.install, // Unknown OS, use default
            };

            if let Some(platform_override) = overrides.get(platform_key) {
                if let Some(install) = &platform_override.install {
                    return install;
                }
            }
        }
        &self.install
    }

    /// Get the appropriate check command for the current platform
    pub fn get_check_command(&self, platform: &Platform) -> &str {
        if let Some(overrides) = &self.overrides {
            let platform_key = match platform.os {
                OS::Windows => "windows",
                OS::Linux => "linux",
                OS::Macos => "macos",
                _ => return &self.check, // Unknown OS, use default
            };

            if let Some(platform_override) = overrides.get(platform_key) {
                if let Some(check) = &platform_override.check {
                    return check;
                }
            }
        }
        &self.check
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_definition_simple_format() {
        // Test simple array format using our custom ccl-parser
        let ccl = r#"
bat =
  = brew
  = scoop
  = pacman
  = nix
"#;
        let packages: HashMap<String, PackageDefinition> = santa_data::parse_ccl_to(ccl).unwrap();
        let def = packages.get("bat").unwrap();

        assert!(def.is_available_in("brew"));
        assert!(def.is_available_in("scoop"));
        assert!(def.is_available_in("pacman"));
        assert!(def.is_available_in("nix"));

        // Simple format should not have source configs
        assert!(def.get_source_config("brew").is_none());

        // Check that all sources are present
        let sources = def.get_sources();
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(&"brew"));
        assert!(sources.contains(&"scoop"));
        assert!(sources.contains(&"pacman"));
        assert!(sources.contains(&"nix"));
    }

    #[test]
    fn test_package_definition_complex_format() {
        let ccl = r#"
ripgrep =
  brew = gh
  _sources =
    = scoop
    = apt
    = pacman
    = nix
"#;
        let packages: HashMap<String, PackageDefinition> = santa_data::parse_ccl_to(ccl).unwrap();
        let def = packages.get("ripgrep").unwrap();

        assert!(def.is_available_in("brew"));
        assert!(def.is_available_in("scoop"));
        assert!(def.get_source_config("brew").is_some());

        // Check that sources list includes all sources
        let sources = def.get_sources();
        assert!(sources.contains(&"scoop"));
        assert!(sources.contains(&"apt"));
        assert!(sources.contains(&"pacman"));
        assert!(sources.contains(&"nix"));
        assert!(sources.contains(&"brew"));
    }

    #[test]
    fn test_source_definition() {
        let ccl = r#"
emoji = 🍺
install = brew install {package}
check = brew leaves --installed-on-request
"#;
        let def: SourceDefinition = sickle::from_str(ccl).unwrap();

        assert_eq!(def.emoji, "🍺");
        assert!(def.install.contains("{package}"));
    }
}

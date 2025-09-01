// Schema-based data structures for Santa Package Manager
// These structs match the YAML schemas defined in /data/*.yaml files

use crate::data::Platform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package definition matching package_schema.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageDefinition {
    /// Simple format: just an array of sources where package has same name
    Simple(Vec<String>),
    /// Complex format: when source-specific config or metadata is needed
    Complex(ComplexPackageDefinition),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl PackageDefinition {
    /// Get all sources where this package is available
    pub fn get_sources(&self) -> Vec<&str> {
        match self {
            PackageDefinition::Simple(sources) => sources.iter().map(|s| s.as_str()).collect(),
            PackageDefinition::Complex(complex) => {
                let mut all_sources = Vec::new();

                // Add sources from _sources array
                if let Some(sources) = &complex.sources {
                    all_sources.extend(sources.iter().map(|s| s.as_str()));
                }

                // Add sources from explicit configurations
                all_sources.extend(complex.source_configs.keys().map(|s| s.as_str()));

                all_sources
            }
        }
    }

    /// Get source-specific configuration for a source
    pub fn get_source_config(&self, source: &str) -> Option<&SourceSpecificConfig> {
        match self {
            PackageDefinition::Simple(_) => None,
            PackageDefinition::Complex(complex) => complex.source_configs.get(source),
        }
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
                crate::data::OS::Windows => "windows",
                crate::data::OS::Linux => "linux",
                crate::data::OS::Macos => "macos",
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
                crate::data::OS::Windows => "windows",
                crate::data::OS::Linux => "linux",
                crate::data::OS::Macos => "macos",
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
    fn test_simple_package_definition() {
        let yaml = r#"[brew, scoop, pacman, nix]"#;
        let def: PackageDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            PackageDefinition::Simple(sources) => {
                assert_eq!(sources.len(), 4);
                assert!(sources.contains(&"brew".to_string()));
            }
            _ => panic!("Expected simple package definition"),
        }
    }

    #[test]
    fn test_complex_package_definition() {
        let yaml = r#"
brew: gh
_sources: [scoop, apt, pacman, nix]
"#;
        let def: PackageDefinition = serde_yaml::from_str(yaml).unwrap();

        assert!(def.is_available_in("brew"));
        assert!(def.is_available_in("scoop"));
        assert!(def.get_source_config("brew").is_some());
    }

    #[test]
    fn test_source_definition() {
        let yaml = r#"
emoji: 🍺
install: brew install {package}
check: brew leaves --installed-on-request
"#;
        let def: SourceDefinition = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(def.emoji, "🍺");
        assert!(def.install.contains("{package}"));
    }
}

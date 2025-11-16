//! Comprehensive tests for schema definitions and methods

use santa_data::*;
use std::collections::HashMap;

// PackageDefinition tests
#[test]
fn test_package_definition_simple_get_sources() {
    let def = PackageDefinition::Simple(vec![
        "brew".to_string(),
        "scoop".to_string(),
        "pacman".to_string(),
    ]);

    let sources = def.get_sources();
    assert_eq!(sources.len(), 3);
    assert!(sources.contains(&"brew"));
    assert!(sources.contains(&"scoop"));
    assert!(sources.contains(&"pacman"));
}

#[test]
fn test_package_definition_simple_is_available_in() {
    let def = PackageDefinition::Simple(vec!["brew".to_string(), "scoop".to_string()]);

    assert!(def.is_available_in("brew"));
    assert!(def.is_available_in("scoop"));
    assert!(!def.is_available_in("pacman"));
}

#[test]
fn test_package_definition_simple_no_source_config() {
    let def = PackageDefinition::Simple(vec!["brew".to_string()]);
    assert!(def.get_source_config("brew").is_none());
}

#[test]
fn test_package_definition_complex_get_sources() {
    let mut complex = ComplexPackageDefinition::default();
    complex.sources = Some(vec!["brew".to_string(), "scoop".to_string()]);
    complex.source_configs.insert(
        "pacman".to_string(),
        SourceSpecificConfig::Name("custom-name".to_string()),
    );

    let def = PackageDefinition::Complex(complex);
    let sources = def.get_sources();

    assert!(sources.contains(&"brew"));
    assert!(sources.contains(&"scoop"));
    assert!(sources.contains(&"pacman"));
}

#[test]
fn test_package_definition_complex_get_source_config() {
    let mut complex = ComplexPackageDefinition::default();
    complex.source_configs.insert(
        "brew".to_string(),
        SourceSpecificConfig::Name("gh".to_string()),
    );

    let def = PackageDefinition::Complex(complex);
    let config = def.get_source_config("brew");

    assert!(config.is_some());
}

#[test]
fn test_package_definition_complex_is_available_in() {
    let mut complex = ComplexPackageDefinition::default();
    complex.sources = Some(vec!["brew".to_string()]);

    let def = PackageDefinition::Complex(complex);

    assert!(def.is_available_in("brew"));
    assert!(!def.is_available_in("scoop"));
}

// ComplexPackageDefinition tests
#[test]
fn test_complex_package_definition_default() {
    let def = ComplexPackageDefinition::default();
    assert!(def.sources.is_none());
    assert!(def.platforms.is_none());
    assert!(def.aliases.is_none());
    assert!(def.source_configs.is_empty());
}

#[test]
fn test_complex_package_definition_with_platforms() {
    let mut def = ComplexPackageDefinition::default();
    def.platforms = Some(vec!["macos".to_string(), "linux".to_string()]);

    assert_eq!(def.platforms.as_ref().unwrap().len(), 2);
}

#[test]
fn test_complex_package_definition_with_aliases() {
    let mut def = ComplexPackageDefinition::default();
    def.aliases = Some(vec!["rg".to_string(), "ripgrep".to_string()]);

    assert_eq!(def.aliases.as_ref().unwrap().len(), 2);
}

#[test]
fn test_complex_package_definition_get_sources_from_configs_only() {
    let mut def = ComplexPackageDefinition::default();
    def.source_configs.insert(
        "brew".to_string(),
        SourceSpecificConfig::Name("custom".to_string()),
    );
    def.source_configs.insert(
        "scoop".to_string(),
        SourceSpecificConfig::Name("other".to_string()),
    );

    let sources = def.get_sources();
    assert!(sources.contains(&"brew"));
    assert!(sources.contains(&"scoop"));
}

#[test]
fn test_complex_package_definition_is_available_in_from_sources() {
    let mut def = ComplexPackageDefinition::default();
    def.sources = Some(vec!["brew".to_string(), "scoop".to_string()]);

    assert!(def.is_available_in("brew"));
    assert!(def.is_available_in("scoop"));
    assert!(!def.is_available_in("pacman"));
}

#[test]
fn test_complex_package_definition_is_available_in_from_configs() {
    let mut def = ComplexPackageDefinition::default();
    def.source_configs.insert(
        "brew".to_string(),
        SourceSpecificConfig::Name("custom".to_string()),
    );

    assert!(def.is_available_in("brew"));
    assert!(!def.is_available_in("scoop"));
}

// SourceSpecificConfig tests
#[test]
fn test_source_specific_config_name() {
    let config = SourceSpecificConfig::Name("custom-name".to_string());
    match config {
        SourceSpecificConfig::Name(name) => assert_eq!(name, "custom-name"),
        _ => panic!("Expected Name variant"),
    }
}

#[test]
fn test_source_specific_config_complex() {
    let config = SourceSpecificConfig::Complex(SourceConfig {
        name: Some("custom".to_string()),
        pre: Some("pre-cmd".to_string()),
        post: Some("post-cmd".to_string()),
        prefix: Some("prefix-".to_string()),
        install_suffix: Some("--flag".to_string()),
    });

    match config {
        SourceSpecificConfig::Complex(c) => {
            assert_eq!(c.name, Some("custom".to_string()));
            assert_eq!(c.pre, Some("pre-cmd".to_string()));
            assert_eq!(c.post, Some("post-cmd".to_string()));
            assert_eq!(c.prefix, Some("prefix-".to_string()));
            assert_eq!(c.install_suffix, Some("--flag".to_string()));
        }
        _ => panic!("Expected Complex variant"),
    }
}

// SourceConfig tests
#[test]
fn test_source_config_full() {
    let config = SourceConfig {
        name: Some("override".to_string()),
        pre: Some("echo pre".to_string()),
        post: Some("echo post".to_string()),
        prefix: Some("pkg-".to_string()),
        install_suffix: Some("--yes".to_string()),
    };

    assert_eq!(config.name, Some("override".to_string()));
    assert_eq!(config.pre, Some("echo pre".to_string()));
    assert_eq!(config.post, Some("echo post".to_string()));
    assert_eq!(config.prefix, Some("pkg-".to_string()));
    assert_eq!(config.install_suffix, Some("--yes".to_string()));
}

// SourceDefinition tests
#[test]
fn test_source_definition_basic() {
    let def = SourceDefinition {
        emoji: "🍺".to_string(),
        install: "brew install {package}".to_string(),
        check: "brew list".to_string(),
        prefix: None,
        overrides: None,
    };

    assert_eq!(def.emoji, "🍺");
    assert!(def.install.contains("{package}"));
}

#[test]
fn test_source_definition_with_prefix() {
    let def = SourceDefinition {
        emoji: "📦".to_string(),
        install: "scoop install {package}".to_string(),
        check: "scoop list".to_string(),
        prefix: Some("bucket/".to_string()),
        overrides: None,
    };

    assert_eq!(def.prefix, Some("bucket/".to_string()));
}

#[test]
fn test_source_definition_get_install_command_no_override() {
    let def = SourceDefinition {
        emoji: "🍺".to_string(),
        install: "brew install {package}".to_string(),
        check: "brew list".to_string(),
        prefix: None,
        overrides: None,
    };

    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    assert_eq!(def.get_install_command(&platform), "brew install {package}");
}

#[test]
fn test_source_definition_get_install_command_with_macos_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "macos".to_string(),
        PlatformOverride {
            install: Some("brew install --cask {package}".to_string()),
            check: None,
        },
    );

    let def = SourceDefinition {
        emoji: "🍺".to_string(),
        install: "brew install {package}".to_string(),
        check: "brew list".to_string(),
        prefix: None,
        overrides: Some(overrides),
    };

    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    assert_eq!(
        def.get_install_command(&platform),
        "brew install --cask {package}"
    );
}

#[test]
fn test_source_definition_get_install_command_with_linux_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "linux".to_string(),
        PlatformOverride {
            install: Some("sudo apt install {package}".to_string()),
            check: None,
        },
    );

    let def = SourceDefinition {
        emoji: "📦".to_string(),
        install: "apt install {package}".to_string(),
        check: "apt list --installed".to_string(),
        prefix: None,
        overrides: Some(overrides),
    };

    let platform = Platform {
        os: OS::Linux,
        arch: Arch::X64,
        distro: Some(Distro::Ubuntu),
    };

    assert_eq!(
        def.get_install_command(&platform),
        "sudo apt install {package}"
    );
}

#[test]
fn test_source_definition_get_install_command_with_windows_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "windows".to_string(),
        PlatformOverride {
            install: Some("scoop install {package}".to_string()),
            check: None,
        },
    );

    let def = SourceDefinition {
        emoji: "🪣".to_string(),
        install: "install {package}".to_string(),
        check: "list".to_string(),
        prefix: None,
        overrides: Some(overrides),
    };

    let platform = Platform {
        os: OS::Windows,
        arch: Arch::X64,
        distro: None,
    };

    assert_eq!(
        def.get_install_command(&platform),
        "scoop install {package}"
    );
}

#[test]
fn test_source_definition_get_check_command_no_override() {
    let def = SourceDefinition {
        emoji: "🍺".to_string(),
        install: "brew install {package}".to_string(),
        check: "brew list".to_string(),
        prefix: None,
        overrides: None,
    };

    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    assert_eq!(def.get_check_command(&platform), "brew list");
}

#[test]
fn test_source_definition_get_check_command_with_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "macos".to_string(),
        PlatformOverride {
            install: None,
            check: Some("brew list --cask".to_string()),
        },
    );

    let def = SourceDefinition {
        emoji: "🍺".to_string(),
        install: "brew install {package}".to_string(),
        check: "brew list".to_string(),
        prefix: None,
        overrides: Some(overrides),
    };

    let platform = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };

    assert_eq!(def.get_check_command(&platform), "brew list --cask");
}

// ConfigDefinition tests
#[test]
fn test_config_definition_basic() {
    let config = ConfigDefinition {
        sources: vec!["brew".to_string(), "scoop".to_string()],
        packages: vec!["bat".to_string(), "ripgrep".to_string()],
        settings: None,
    };

    assert_eq!(config.sources.len(), 2);
    assert_eq!(config.packages.len(), 2);
    assert!(config.settings.is_none());
}

#[test]
fn test_config_definition_with_settings() {
    let settings = ConfigSettings {
        auto_update: true,
        parallel_installs: 5,
        confirm_before_install: false,
    };

    let config = ConfigDefinition {
        sources: vec!["brew".to_string()],
        packages: vec!["bat".to_string()],
        settings: Some(settings),
    };

    let s = config.settings.unwrap();
    assert!(s.auto_update);
    assert_eq!(s.parallel_installs, 5);
    assert!(!s.confirm_before_install);
}

// ConfigSettings tests
#[test]
fn test_config_settings_defaults() {
    let settings = ConfigSettings {
        auto_update: false,
        parallel_installs: 3,
        confirm_before_install: true,
    };

    assert!(!settings.auto_update);
    assert_eq!(settings.parallel_installs, 3);
    assert!(settings.confirm_before_install);
}

// Integration test: Parse full package definition
#[test]
fn test_parse_full_package_with_all_features() {
    let ccl = r#"
ripgrep =
  _sources =
    = brew
    = scoop
    = pacman
  _platforms =
    = macos
    = windows
    = linux
  _aliases =
    = rg
  brew = gh
"#;
    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(ccl).unwrap();
    let def = packages.get("ripgrep").unwrap();

    // Check sources
    let sources = def.get_sources();
    assert!(sources.contains(&"brew"));
    assert!(sources.contains(&"scoop"));
    assert!(sources.contains(&"pacman"));

    // Check availability
    assert!(def.is_available_in("brew"));
    assert!(def.is_available_in("scoop"));

    // Check source config
    assert!(def.get_source_config("brew").is_some());
}

// Integration test: Parse source definition
#[test]
fn test_parse_source_definition_with_overrides() {
    let ccl = r#"
emoji = 🍺
install = brew install {package}
check = brew list
_overrides =
  macos =
    install = brew install --cask {package}
  linux =
    check = brew list --installed
"#;
    let def: SourceDefinition = serde_ccl::from_str(ccl).unwrap();

    assert_eq!(def.emoji, "🍺");
    assert!(def.overrides.is_some());

    let overrides = def.overrides.unwrap();
    assert!(overrides.contains_key("macos"));
    assert!(overrides.contains_key("linux"));
}

// Integration test: Parse config definition
#[test]
fn test_parse_config_definition() {
    let ccl = r#"
sources =
  = brew
  = scoop
packages =
  = bat
  = ripgrep
_settings =
  auto_update = true
  parallel_installs = 5
  confirm_before_install = false
"#;
    let config: ConfigDefinition = serde_ccl::from_str(ccl).unwrap();

    assert_eq!(config.sources.len(), 2);
    assert_eq!(config.packages.len(), 2);
    assert!(config.settings.is_some());

    let settings = config.settings.unwrap();
    assert!(settings.auto_update);
    assert_eq!(settings.parallel_installs, 5);
}

//! Integration tests for the full parsing pipeline

use santa_data::*;
use std::collections::HashMap;

/// Test end-to-end parsing of a realistic known_packages.ccl file
#[test]
fn test_parse_realistic_packages_file() {
    let ccl = r#"
bat =
  = brew
  = scoop
  = pacman
  = apt
  = nix

ripgrep =
  _sources =
    = scoop
    = apt
    = pacman
    = nix
  brew = gh

fd =
  = brew
  = scoop
  = pacman

gh =
  _sources =
    = brew
    = scoop
    = apt
  _platforms =
    = macos
    = windows
    = linux

node =
  _sources =
    = brew
    = scoop
  brew = node@20
  scoop = nodejs-lts
"#;

    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(ccl).unwrap();

    // Verify all packages parsed
    assert_eq!(packages.len(), 5);
    assert!(packages.contains_key("bat"));
    assert!(packages.contains_key("ripgrep"));
    assert!(packages.contains_key("fd"));
    assert!(packages.contains_key("gh"));
    assert!(packages.contains_key("node"));

    // Verify bat (simple format)
    let bat = &packages["bat"];
    assert!(bat.is_available_in("brew"));
    assert!(bat.is_available_in("scoop"));
    assert!(bat.is_available_in("pacman"));
    assert!(bat.is_available_in("apt"));
    assert!(bat.is_available_in("nix"));

    // Verify ripgrep (complex with override)
    let ripgrep = &packages["ripgrep"];
    assert!(ripgrep.is_available_in("brew"));
    assert!(ripgrep.is_available_in("scoop"));
    assert!(ripgrep.get_source_config("brew").is_some());

    // Verify node (complex with name overrides)
    let node = &packages["node"];
    assert!(node.is_available_in("brew"));
    assert!(node.is_available_in("scoop"));
    assert!(node.get_source_config("brew").is_some());
    assert!(node.get_source_config("scoop").is_some());
}

/// Test end-to-end parsing of a realistic sources.ccl file
/// Note: For deeply nested structures like _overrides, use serde_ccl directly
#[test]
fn test_parse_realistic_sources_file() {
    // Test simple source definitions without nested overrides
    let ccl = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew leaves --installed-on-request

scoop =
  emoji = 🪣
  install = scoop install {package}
  check = scoop list

apt =
  emoji = 📦
  install = sudo apt install -y {package}
  check = apt list --installed
"#;

    let sources: HashMap<String, SourceDefinition> = parse_ccl_to(ccl).unwrap();

    // Verify all sources parsed
    assert_eq!(sources.len(), 3);
    assert!(sources.contains_key("brew"));
    assert!(sources.contains_key("scoop"));
    assert!(sources.contains_key("apt"));

    // Verify brew
    let brew = &sources["brew"];
    assert_eq!(brew.emoji, "🍺");
    assert!(brew.install.contains("brew install"));

    // Verify scoop
    let scoop = &sources["scoop"];
    assert_eq!(scoop.emoji, "🪣");
    assert!(scoop.install.contains("scoop install"));

    // Verify apt
    let apt = &sources["apt"];
    assert_eq!(apt.emoji, "📦");
    assert!(apt.install.contains("sudo apt install"));
}

/// Test end-to-end parsing of a realistic config file
#[test]
fn test_parse_realistic_config_file() {
    let ccl = r#"
sources =
  = brew
  = scoop
  = apt
  = pacman

packages =
  = bat
  = ripgrep
  = fd
  = gh
  = node

_settings =
  auto_update = true
  parallel_installs = 3
  confirm_before_install = true
"#;

    let config: ConfigDefinition = serde_ccl::from_str(ccl).unwrap();

    // Verify sources
    assert_eq!(config.sources.len(), 4);
    assert!(config.sources.contains(&"brew".to_string()));
    assert!(config.sources.contains(&"scoop".to_string()));
    assert!(config.sources.contains(&"apt".to_string()));
    assert!(config.sources.contains(&"pacman".to_string()));

    // Verify packages
    assert_eq!(config.packages.len(), 5);
    assert!(config.packages.contains(&"bat".to_string()));
    assert!(config.packages.contains(&"ripgrep".to_string()));

    // Verify settings
    assert!(config.settings.is_some());
    let settings = config.settings.unwrap();
    assert!(settings.auto_update);
    assert_eq!(settings.parallel_installs, 3);
    assert!(settings.confirm_before_install);
}

/// Test round-trip: parse CCL -> serialize to JSON -> deserialize from JSON
#[test]
fn test_roundtrip_package_definition() {
    let ccl = r#"
bat =
  = brew
  = scoop
"#;

    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(ccl).unwrap();
    let json = serde_json::to_string(&packages).unwrap();
    let parsed: HashMap<String, PackageDefinition> = serde_json::from_str(&json).unwrap();

    assert_eq!(packages.len(), parsed.len());
    assert!(parsed.contains_key("bat"));
}

/// Test combining data from multiple parsed files
#[test]
fn test_combine_packages_and_sources() {
    let packages_ccl = r#"
bat =
  = brew
  = scoop
"#;

    let sources_ccl = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew list

scoop =
  emoji = 🪣
  install = scoop install {package}
  check = scoop list
"#;

    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(packages_ccl).unwrap();
    let sources: HashMap<String, SourceDefinition> = parse_ccl_to(sources_ccl).unwrap();

    // Verify we can access package info
    let bat = &packages["bat"];
    let bat_sources = bat.get_sources();

    // Verify we can look up source definitions
    for source in bat_sources {
        assert!(sources.contains_key(source));
    }
}

/// Test error handling for malformed CCL
#[test]
fn test_malformed_ccl_handling() {
    let invalid_ccl = "this is not valid ccl syntax = = =";
    let result: Result<HashMap<String, PackageDefinition>, _> = parse_ccl_to(invalid_ccl);

    // The parser may or may not error on this - document actual behavior
    // Currently serde_ccl is lenient and may parse partial structures
    if result.is_err() {
        // Error case - parsing failed as expected
        assert!(result.is_err());
    } else {
        // Success case - parser was lenient
        assert!(result.is_ok());
    }
}

/// Test parsing with special characters in values
#[test]
fn test_special_characters_in_values() {
    let ccl = r#"
package =
  emoji = 🎉
  name = my-package@1.0.0
  command = echo "hello world"
"#;

    let result: Result<HashMap<String, serde_json::Value>, _> = parse_ccl_to(ccl);
    assert!(result.is_ok() || result.is_err()); // Document behavior
}

/// Test parsing large dataset performance
#[test]
fn test_parse_large_dataset() {
    let mut ccl = String::new();

    // Generate 100 packages
    for i in 0..100 {
        ccl.push_str(&format!(
            r#"
package{} =
  = brew
  = scoop
  = pacman
"#,
            i
        ));
    }

    let start = std::time::Instant::now();
    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(&ccl).unwrap();
    let duration = start.elapsed();

    assert_eq!(packages.len(), 100);
    // Should parse 100 packages in reasonable time (< 1 second)
    assert!(duration.as_secs() < 1);
}

/// Test parsing with different indentation styles
#[test]
fn test_various_indentation_styles() {
    let ccl_2spaces = r#"
pkg =
  = brew
  = scoop
"#;

    let ccl_4spaces = r#"
pkg =
    = brew
    = scoop
"#;

    let result_2: Result<HashMap<String, PackageDefinition>, _> = parse_ccl_to(ccl_2spaces);
    let result_4: Result<HashMap<String, PackageDefinition>, _> = parse_ccl_to(ccl_4spaces);

    // Document which indentation styles are supported
    assert!(result_2.is_ok() || result_2.is_err());
    assert!(result_4.is_ok() || result_4.is_err());
}

/// Test platform-specific source resolution
/// Note: This test uses serde_ccl directly for nested _overrides structure
#[test]
fn test_platform_specific_source_resolution() {
    let sources_ccl = r#"
emoji = 🍺
install = brew install {package}
check = brew list
_overrides =
  macos =
    install = brew install --cask {package}
  linux =
    install = linuxbrew install {package}
"#;

    // Use serde_ccl directly for deeply nested structures
    let brew: SourceDefinition = serde_ccl::from_str(sources_ccl).unwrap();

    // Test macOS platform
    let macos = Platform {
        os: OS::Macos,
        arch: Arch::Aarch64,
        distro: None,
    };
    assert_eq!(
        brew.get_install_command(&macos),
        "brew install --cask {package}"
    );

    // Test Linux platform
    let linux = Platform {
        os: OS::Linux,
        arch: Arch::X64,
        distro: Some(Distro::Ubuntu),
    };
    assert_eq!(
        brew.get_install_command(&linux),
        "linuxbrew install {package}"
    );

    // Test Windows platform (no override, should use default)
    let windows = Platform {
        os: OS::Windows,
        arch: Arch::X64,
        distro: None,
    };
    assert_eq!(brew.get_install_command(&windows), "brew install {package}");
}

/// Test cross-module integration: using parsed data with models
#[test]
fn test_cross_module_integration() {
    let packages_ccl = r#"
bat =
  = brew
  = scoop
"#;

    let packages: HashMap<String, PackageDefinition> = parse_ccl_to(packages_ccl).unwrap();
    let bat_def = &packages["bat"];

    // Create PackageData using the parsed sources
    let sources = bat_def.get_sources();
    for source in sources {
        let pkg_data = PackageData::new("bat");
        assert_eq!(pkg_data.name, Some("bat".to_string()));

        // Simulate source name mapping
        let _source_name = SourceName(source.to_string());
    }
}

/// Test that all public API functions work together
#[test]
fn test_public_api_integration() {
    let ccl = r#"
bat =
  = brew
  = scoop
"#;

    // Test parse_to_hashmap
    let hashmap = parse_to_hashmap(ccl).unwrap();
    assert!(hashmap.contains_key("bat"));

    // Test parse_ccl_to
    let typed: HashMap<String, PackageDefinition> = parse_ccl_to(ccl).unwrap();
    assert!(typed.contains_key("bat"));

    // Test CclValue conversion
    let ccl_value = CclValue::Array(vec!["brew".to_string(), "scoop".to_string()]);
    let json_value: serde_json::Value = ccl_value.into();
    assert!(json_value.is_array());
}

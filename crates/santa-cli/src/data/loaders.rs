// Data loading functions using the new schema-based structures

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

#[cfg(test)]
use super::schemas::ComplexPackageDefinition;
use super::schemas::{ConfigDefinition, PackageDefinition, SourcesDefinition};
use crate::data::{KnownSources, PackageData, PackageDataList, SourceList};
use crate::sources::PackageSource;

/// Load packages from the new schema format
/// Supports both simple array format and complex object format
pub fn load_packages_from_schema(path: &Path) -> Result<HashMap<String, PackageDefinition>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read packages file: {:?}", path))?;

    // Use our custom CCL parser that handles both simple and complex formats
    let packages: HashMap<String, PackageDefinition> = santa_data::parse_ccl_to(&content)
        .with_context(|| format!("Failed to parse CCL packages: {:?}", path))?;

    info!("Loaded {} packages from schema format", packages.len());
    Ok(packages)
}

/// Load sources from the new schema format
pub fn load_sources_from_schema(path: &Path) -> Result<SourcesDefinition> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read sources file: {:?}", path))?;

    let sources: SourcesDefinition = serde_ccl::from_str(&content)
        .with_context(|| format!("Failed to parse CCL sources: {:?}", path))?;

    info!("Loaded {} sources from schema format", sources.len());
    Ok(sources)
}

/// Load configuration from the new schema format
pub fn load_config_from_schema(path: &Path) -> Result<ConfigDefinition> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let config: ConfigDefinition = serde_ccl::from_str(&content)
        .with_context(|| format!("Failed to parse CCL config: {:?}", path))?;

    info!(
        "Loaded config with {} sources and {} packages",
        config.sources.len(),
        config.packages.len()
    );
    Ok(config)
}

/// Convert new schema packages to legacy PackageDataList format
/// This provides backward compatibility while we migrate the codebase
pub fn convert_to_legacy_packages(
    schema_packages: HashMap<String, PackageDefinition>,
) -> PackageDataList {
    let mut legacy_packages = PackageDataList::new();

    for (package_name, package_def) in schema_packages {
        let mut source_map = HashMap::new();

        // Get all sources for this package
        let sources = package_def.get_sources();

        for source_name in sources {
            // Try to parse as KnownSources
            let known_source = match source_name {
                "brew" => KnownSources::Brew,
                "scoop" => KnownSources::Scoop,
                "pacman" => KnownSources::Pacman,
                "nix" => KnownSources::Nix,
                "cargo" => KnownSources::Cargo,
                "apt" => KnownSources::Apt,
                "aur" => KnownSources::Aur,
                other => {
                    warn!(
                        "Unknown source '{}' for package '{}', skipping",
                        other, package_name
                    );
                    continue;
                }
            };

            // Create PackageData based on source configuration
            let package_data = if let Some(source_config) =
                package_def.get_source_config(source_name)
            {
                match source_config {
                    super::schemas::SourceSpecificConfig::Name(name) => Some(PackageData {
                        name: Some(name.clone()),
                        before: None,
                        after: None,
                        pre: None,
                        post: None,
                    }),
                    super::schemas::SourceSpecificConfig::Complex(config) => Some(PackageData {
                        name: config.name.clone(),
                        before: config.pre.clone(),
                        after: config.post.clone(),
                        pre: config.prefix.clone(),
                        post: config.install_suffix.clone(),
                    }),
                }
            } else {
                // No specific config, package uses same name
                None
            };

            source_map.insert(known_source, package_data);
        }

        if !source_map.is_empty() {
            legacy_packages.insert(package_name, source_map);
        }
    }

    legacy_packages
}

/// Convert new schema sources to legacy SourceList format
pub fn convert_to_legacy_sources(schema_sources: SourcesDefinition) -> SourceList {
    let mut legacy_sources = SourceList::new();

    for (source_name, source_def) in schema_sources {
        let known_source = match source_name.as_str() {
            "brew" => KnownSources::Brew,
            "scoop" => KnownSources::Scoop,
            "pacman" => KnownSources::Pacman,
            "nix" => KnownSources::Nix,
            "cargo" => KnownSources::Cargo,
            "apt" => KnownSources::Apt,
            "aur" => KnownSources::Aur,
            other => {
                warn!("Unknown source '{}', skipping", other);
                continue;
            }
        };

        // Create PackageSource from schema using new_for_test (TODO: create proper constructor)
        let package_source = PackageSource::new_for_test(
            known_source,
            &source_def.emoji,
            &source_name, // Use source name as shell command
            &source_def.install,
            &source_def.check,
            source_def.prefix.clone(),
            None, // TODO: Convert platform overrides
        );

        legacy_sources.push(package_source);
    }

    legacy_sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_debug_ccl_parsing() {
        println!("\n=== DEBUGGING SERDE_CCL PARSING ===\n");

        // Test 1: What does serde_ccl return for a raw array?
        let array_ccl = r#"
  = brew
  = scoop
"#;
        println!("--- Test 1: Raw array ---");
        println!("Input CCL:\n{}", array_ccl);

        let vec_result: Result<Vec<String>, _> = serde_ccl::from_str(array_ccl);
        println!("Vec<String> result: {:?}\n", vec_result);

        let json_value: Result<serde_json::Value, _> = serde_ccl::from_str(array_ccl);
        println!("serde_json::Value result: {:?}\n", json_value);

        // Test 2: What about in a HashMap context?
        let full_simple = r#"
test_pkg =
  = brew
  = scoop
"#;
        println!("--- Test 2: HashMap with simple array ---");
        println!("Input CCL:\n{}", full_simple);

        // Parse as generic Value to see structure
        let json_result: Result<serde_json::Value, _> = serde_ccl::from_str(full_simple);
        println!("As serde_json::Value: {:#?}\n", json_result);

        // Parse as HashMap<String, Value>
        let hash_value: Result<HashMap<String, serde_json::Value>, _> =
            serde_ccl::from_str(full_simple);
        println!("As HashMap<String, Value>: {:#?}\n", hash_value);

        // What type does the value have?
        if let Ok(ref map) = hash_value {
            if let Some(value) = map.get("test_pkg") {
                println!("Value type for 'test_pkg': ");
                println!("  is_string: {}", value.is_string());
                println!("  is_array: {}", value.is_array());
                println!("  is_object: {}", value.is_object());
                println!("  is_null: {}", value.is_null());
                println!("  is_boolean: {}", value.is_boolean());
                println!("  is_number: {}", value.is_number());
                println!("  actual value: {:#?}\n", value);
            }
        }

        // Test 3: Compare with complex format
        let full_complex = r#"
test_pkg =
  _sources =
    = brew
    = scoop
"#;
        println!("--- Test 3: HashMap with complex format ---");
        println!("Input CCL:\n{}", full_complex);

        let complex_json: Result<serde_json::Value, _> = serde_ccl::from_str(full_complex);
        println!("As serde_json::Value: {:#?}\n", complex_json);

        let complex_hash: Result<HashMap<String, serde_json::Value>, _> =
            serde_ccl::from_str(full_complex);
        println!("As HashMap<String, Value>: {:#?}\n", complex_hash);

        if let Ok(ref map) = complex_hash {
            if let Some(value) = map.get("test_pkg") {
                println!("Value type for 'test_pkg': ");
                println!("  is_string: {}", value.is_string());
                println!("  is_array: {}", value.is_array());
                println!("  is_object: {}", value.is_object());
                println!("  actual value: {:#?}\n", value);
            }
        }

        println!("=== END DEBUGGING ===\n");
    }

    #[test]
    fn test_load_simple_array_format() {
        // Test loading packages in simple array format using our custom ccl-parser
        // that works around serde_ccl limitations

        let ccl_content = r#"
bat =
  = brew
  = scoop
  = pacman
  = nix

fd =
  = brew
  = scoop
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ccl_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let packages = load_packages_from_schema(temp_file.path()).unwrap();

        assert_eq!(packages.len(), 2);
        assert!(packages.contains_key("bat"));
        assert!(packages.contains_key("fd"));

        // Test simple array format
        let bat = &packages["bat"];
        assert!(bat.is_available_in("brew"));
        assert!(bat.is_available_in("scoop"));
        assert!(bat.is_available_in("pacman"));
        assert!(bat.is_available_in("nix"));
        assert_eq!(bat.get_sources().len(), 4);

        let fd = &packages["fd"];
        assert!(fd.is_available_in("brew"));
        assert!(fd.is_available_in("scoop"));
        assert_eq!(fd.get_sources().len(), 2);
    }

    #[test]
    fn test_load_complex_packages() {
        let ccl_content = r#"
bat =
  _sources =
    = brew
    = scoop
    = pacman
    = nix

ripgrep =
  brew = rg
  _sources =
    = scoop
    = pacman
    = nix
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ccl_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let packages = load_packages_from_schema(temp_file.path()).unwrap();

        assert_eq!(packages.len(), 2);
        assert!(packages.contains_key("bat"));
        assert!(packages.contains_key("ripgrep"));

        // Test complex format with _sources
        let bat = &packages["bat"];
        assert!(bat.is_available_in("brew"));
        assert!(bat.is_available_in("nix"));

        // Test complex format with source override
        let ripgrep = &packages["ripgrep"];
        assert!(ripgrep.is_available_in("brew"));
        assert!(ripgrep.is_available_in("scoop"));
        assert!(ripgrep.get_source_config("brew").is_some());
    }

    #[test]
    fn test_load_sources() {
        let ccl_content = r#"
brew =
  emoji = 🍺
  install = brew install {package}
  check = brew leaves --installed-on-request

npm =
  emoji = 📦
  install = npm install -g {package}
  check = ls -1 `npm root -g`
  _overrides =
    windows =
      check = npm root -g | gci -Name
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ccl_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let sources = load_sources_from_schema(temp_file.path()).unwrap();

        assert_eq!(sources.len(), 2);
        assert!(sources.contains_key("brew"));
        assert!(sources.contains_key("npm"));

        let brew = &sources["brew"];
        assert_eq!(brew.emoji, "🍺");
        assert!(brew.install.contains("{package}"));

        let npm = &sources["npm"];
        assert!(npm.overrides.is_some());
    }

    #[test]
    fn test_convert_simple_format_to_legacy() {
        let mut schema_packages = HashMap::new();
        // Simple format: just an array of sources
        schema_packages.insert(
            "bat".to_string(),
            PackageDefinition::Simple(vec!["brew".to_string(), "scoop".to_string()]),
        );

        let legacy = convert_to_legacy_packages(schema_packages);

        assert!(legacy.contains_key("bat"));
        let bat_sources = &legacy["bat"];
        assert!(bat_sources.contains_key(&KnownSources::Brew));
        assert!(bat_sources.contains_key(&KnownSources::Scoop));

        // Simple format should have None for package data (same name)
        assert_eq!(bat_sources.get(&KnownSources::Brew), Some(&None));
        assert_eq!(bat_sources.get(&KnownSources::Scoop), Some(&None));
    }

    #[test]
    fn test_convert_complex_format_to_legacy() {
        let mut schema_packages = HashMap::new();
        schema_packages.insert(
            "bat".to_string(),
            PackageDefinition::Complex(ComplexPackageDefinition {
                sources: Some(vec!["brew".to_string(), "scoop".to_string()]),
                platforms: None,
                aliases: None,
                source_configs: HashMap::new(),
            }),
        );

        let legacy = convert_to_legacy_packages(schema_packages);

        assert!(legacy.contains_key("bat"));
        let bat_sources = &legacy["bat"];
        assert!(bat_sources.contains_key(&KnownSources::Brew));
        assert!(bat_sources.contains_key(&KnownSources::Scoop));
    }
}

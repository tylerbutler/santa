// Data loading functions using the new schema-based structures

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use tracing::{info, warn};

use hocon::HoconLoader;

use super::schemas::{PackageDefinition, SourcesDefinition, ConfigDefinition};
use crate::data::{KnownSources, PackageData, PackageDataList, SourceList};
use crate::sources::PackageSource;

/// Load packages from the new schema format
pub fn load_packages_from_schema(path: &Path) -> Result<HashMap<String, PackageDefinition>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read packages file: {:?}", path))?;
    
    let packages: HashMap<String, PackageDefinition> = HoconLoader::new()
        .load_str(&content)?
        .resolve()
        .with_context(|| format!("Failed to parse HOCON packages: {:?}", path))?;
    
    info!("Loaded {} packages from schema format", packages.len());
    Ok(packages)
}

/// Load sources from the new schema format
pub fn load_sources_from_schema(path: &Path) -> Result<SourcesDefinition> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read sources file: {:?}", path))?;
    
    let sources: SourcesDefinition = HoconLoader::new()
        .load_str(&content)?
        .resolve()
        .with_context(|| format!("Failed to parse HOCON sources: {:?}", path))?;
    
    info!("Loaded {} sources from schema format", sources.len());
    Ok(sources)
}

/// Load configuration from the new schema format
pub fn load_config_from_schema(path: &Path) -> Result<ConfigDefinition> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;
    
    let config: ConfigDefinition = HoconLoader::new()
        .load_str(&content)?
        .resolve()
        .with_context(|| format!("Failed to parse HOCON config: {:?}", path))?;
    
    info!("Loaded config with {} sources and {} packages", 
          config.sources.len(), config.packages.len());
    Ok(config)
}

/// Convert new schema packages to legacy PackageDataList format
/// This provides backward compatibility while we migrate the codebase
pub fn convert_to_legacy_packages(
    schema_packages: HashMap<String, PackageDefinition>
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
                    warn!("Unknown source '{}' for package '{}', skipping", other, package_name);
                    continue;
                }
            };
            
            // Create PackageData based on source configuration
            let package_data = if let Some(source_config) = package_def.get_source_config(source_name) {
                match source_config {
                    super::schemas::SourceSpecificConfig::Name(name) => {
                        Some(PackageData {
                            name: Some(name.clone()),
                            before: None,
                            after: None, 
                            pre: None,
                            post: None,
                        })
                    }
                    super::schemas::SourceSpecificConfig::Complex(config) => {
                        Some(PackageData {
                            name: config.name.clone(),
                            before: config.pre.clone(),
                            after: config.post.clone(),
                            pre: config.prefix.clone(),
                            post: config.install_suffix.clone(),
                        })
                    }
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
    fn test_load_simple_packages() {
        let yaml_content = r#"
bat: [brew, scoop, pacman, nix]
ripgrep:
  brew: rg
  _sources: [scoop, pacman, nix]
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();
        
        let packages = load_packages_from_schema(temp_file.path()).unwrap();
        
        assert_eq!(packages.len(), 2);
        assert!(packages.contains_key("bat"));
        assert!(packages.contains_key("ripgrep"));
        
        // Test simple format
        let bat = &packages["bat"];
        assert!(bat.is_available_in("brew"));
        assert!(bat.is_available_in("nix"));
        
        // Test complex format
        let ripgrep = &packages["ripgrep"];
        assert!(ripgrep.is_available_in("brew"));
        assert!(ripgrep.is_available_in("scoop"));
        assert!(ripgrep.get_source_config("brew").is_some());
    }
    
    #[test]
    fn test_load_sources() {
        let yaml_content = r#"
brew:
  emoji: 🍺
  install: brew install {package}
  check: brew leaves --installed-on-request
  
npm:
  emoji: 📦
  install: npm install -g {package}
  check: ls -1 `npm root -g`
  _overrides:
    windows:
      check: npm root -g | gci -Name
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();
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
    fn test_convert_to_legacy() {
        let mut schema_packages = HashMap::new();
        schema_packages.insert(
            "bat".to_string(), 
            PackageDefinition::Simple(vec!["brew".to_string(), "scoop".to_string()])
        );
        
        let legacy = convert_to_legacy_packages(schema_packages);
        
        assert!(legacy.contains_key("bat"));
        let bat_sources = &legacy["bat"];
        assert!(bat_sources.contains_key(&KnownSources::Brew));
        assert!(bat_sources.contains_key(&KnownSources::Scoop));
    }
}
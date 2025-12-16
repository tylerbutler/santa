//! Source layering system for Santa
//!
//! This module handles the three-layer source configuration:
//! 1. **Bundled** - Sources compiled into the binary
//! 2. **Downloaded** - Sources fetched from GitHub (stored locally)
//! 3. **User custom** - User-defined sources in their config (highest priority)
//!
//! Sources with the same name are completely overridden by higher-priority layers.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::data::schemas::SourcesDefinition;

/// URL to fetch the latest sources from GitHub
pub const SOURCES_URL: &str =
    "https://raw.githubusercontent.com/tylerbutler/santa/main/crates/santa-cli/data/sources.ccl";

/// Filename for downloaded sources
pub const DOWNLOADED_SOURCES_FILENAME: &str = "sources.ccl";

/// Represents the origin of a source definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceOrigin {
    /// Compiled into the binary
    Bundled,
    /// Downloaded from GitHub
    Downloaded,
    /// User-defined in config
    UserCustom,
}

impl std::fmt::Display for SourceOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceOrigin::Bundled => write!(f, "bundled"),
            SourceOrigin::Downloaded => write!(f, "downloaded"),
            SourceOrigin::UserCustom => write!(f, "custom"),
        }
    }
}

/// A source with its origin tracked
#[derive(Debug, Clone)]
pub struct LayeredSource {
    pub name: String,
    pub definition: crate::data::schemas::SourceDefinition,
    pub origin: SourceOrigin,
}

/// Manages the layered source system
pub struct SourceLayerManager {
    config_dir: PathBuf,
}

impl SourceLayerManager {
    /// Create a new SourceLayerManager with the given config directory
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Path to the downloaded sources file
    pub fn downloaded_sources_path(&self) -> PathBuf {
        self.config_dir.join(DOWNLOADED_SOURCES_FILENAME)
    }

    /// Load downloaded sources if they exist
    pub fn load_downloaded_sources(&self) -> Result<Option<SourcesDefinition>> {
        let path = self.downloaded_sources_path();
        if !path.exists() {
            debug!("No downloaded sources found at {:?}", path);
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read downloaded sources from {:?}", path))?;

        let sources: SourcesDefinition = sickle::from_str(&content)
            .with_context(|| format!("Failed to parse downloaded sources from {:?}", path))?;

        debug!("Loaded {} downloaded sources", sources.len());
        Ok(Some(sources))
    }

    /// Load bundled sources (compiled into binary)
    pub fn load_bundled_sources(&self) -> Result<SourcesDefinition> {
        let content = include_str!("../data/sources.ccl");
        let sources: SourcesDefinition =
            sickle::from_str(content).context("Failed to parse bundled sources")?;
        debug!("Loaded {} bundled sources", sources.len());
        Ok(sources)
    }

    /// Merge sources from all layers
    ///
    /// Priority (highest to lowest):
    /// 1. User custom sources (from config)
    /// 2. Downloaded sources
    /// 3. Bundled sources
    pub fn merge_sources(
        &self,
        user_custom: Option<&SourcesDefinition>,
    ) -> Result<Vec<LayeredSource>> {
        let mut result: HashMap<String, LayeredSource> = HashMap::new();

        // Layer 1: Bundled (lowest priority)
        let bundled = self.load_bundled_sources()?;
        for (name, definition) in bundled {
            result.insert(
                name.clone(),
                LayeredSource {
                    name,
                    definition,
                    origin: SourceOrigin::Bundled,
                },
            );
        }

        // Layer 2: Downloaded (overrides bundled)
        if let Some(downloaded) = self.load_downloaded_sources()? {
            for (name, definition) in downloaded {
                if result.contains_key(&name) {
                    debug!("Downloaded source '{}' overrides bundled", name);
                }
                result.insert(
                    name.clone(),
                    LayeredSource {
                        name,
                        definition,
                        origin: SourceOrigin::Downloaded,
                    },
                );
            }
        }

        // Layer 3: User custom (highest priority)
        if let Some(custom) = user_custom {
            for (name, definition) in custom {
                if result.contains_key(name) {
                    debug!("User custom source '{}' overrides lower layers", name);
                }
                result.insert(
                    name.clone(),
                    LayeredSource {
                        name: name.clone(),
                        definition: definition.clone(),
                        origin: SourceOrigin::UserCustom,
                    },
                );
            }
        }

        // Sort by name for consistent ordering
        let mut sources: Vec<LayeredSource> = result.into_values().collect();
        sources.sort_by(|a, b| a.name.cmp(&b.name));

        info!("Merged {} total sources from all layers", sources.len());
        Ok(sources)
    }

    /// Get a summary of sources by origin
    pub fn sources_summary(&self, sources: &[LayeredSource]) -> HashMap<SourceOrigin, usize> {
        let mut summary = HashMap::new();
        for source in sources {
            *summary.entry(source.origin).or_insert(0) += 1;
        }
        summary
    }

    /// Check if downloaded sources exist
    pub fn has_downloaded_sources(&self) -> bool {
        self.downloaded_sources_path().exists()
    }

    /// Delete downloaded sources
    pub fn clear_downloaded_sources(&self) -> Result<()> {
        let path = self.downloaded_sources_path();
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("Failed to remove {:?}", path))?;
            info!("Removed downloaded sources at {:?}", path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SourceLayerManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = SourceLayerManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_load_bundled_sources() {
        let (manager, _temp) = create_test_manager();
        let sources = manager.load_bundled_sources().unwrap();
        assert!(!sources.is_empty(), "Should have bundled sources");
        assert!(sources.contains_key("brew"), "Should have brew source");
    }

    #[test]
    fn test_no_downloaded_sources_initially() {
        let (manager, _temp) = create_test_manager();
        assert!(!manager.has_downloaded_sources());
        let downloaded = manager.load_downloaded_sources().unwrap();
        assert!(downloaded.is_none());
    }

    #[test]
    fn test_merge_bundled_only() {
        let (manager, _temp) = create_test_manager();
        let merged = manager.merge_sources(None).unwrap();
        assert!(!merged.is_empty());

        // All should be bundled origin
        for source in &merged {
            assert_eq!(source.origin, SourceOrigin::Bundled);
        }
    }

    #[test]
    fn test_merge_with_user_custom_override() {
        let (manager, _temp) = create_test_manager();

        // Create a custom source that overrides brew
        let mut custom = SourcesDefinition::new();
        custom.insert(
            "brew".to_string(),
            crate::data::schemas::SourceDefinition {
                emoji: "🍻".to_string(), // Different emoji
                install: "custom-brew install {package}".to_string(),
                check: "custom-brew list".to_string(),
                prefix: None,
                overrides: None,
            },
        );

        let merged = manager.merge_sources(Some(&custom)).unwrap();

        // Find brew in merged sources
        let brew = merged.iter().find(|s| s.name == "brew").unwrap();
        assert_eq!(brew.origin, SourceOrigin::UserCustom);
        assert_eq!(brew.definition.emoji, "🍻");
    }

    #[test]
    fn test_merge_with_new_user_source() {
        let (manager, _temp) = create_test_manager();

        // Create a completely new source
        let mut custom = SourcesDefinition::new();
        custom.insert(
            "my-custom-pm".to_string(),
            crate::data::schemas::SourceDefinition {
                emoji: "🔧".to_string(),
                install: "my-pm install {package}".to_string(),
                check: "my-pm list".to_string(),
                prefix: None,
                overrides: None,
            },
        );

        let merged = manager.merge_sources(Some(&custom)).unwrap();

        // Should have the new source
        let custom_source = merged.iter().find(|s| s.name == "my-custom-pm").unwrap();
        assert_eq!(custom_source.origin, SourceOrigin::UserCustom);

        // Should still have bundled sources
        let brew = merged.iter().find(|s| s.name == "brew").unwrap();
        assert_eq!(brew.origin, SourceOrigin::Bundled);
    }

    #[test]
    fn test_downloaded_sources_layer() {
        let (manager, temp) = create_test_manager();

        // Create downloaded sources file
        let downloaded_content = r#"
brew =
  emoji = 🍺🍺
  install = downloaded-brew install {package}
  check = downloaded-brew list
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_sources_path(), downloaded_content).unwrap();

        assert!(manager.has_downloaded_sources());

        let merged = manager.merge_sources(None).unwrap();
        let brew = merged.iter().find(|s| s.name == "brew").unwrap();

        // Should be from downloaded layer
        assert_eq!(brew.origin, SourceOrigin::Downloaded);
        assert_eq!(brew.definition.emoji, "🍺🍺");
    }

    #[test]
    fn test_user_custom_overrides_downloaded() {
        let (manager, temp) = create_test_manager();

        // Create downloaded sources file
        let downloaded_content = r#"
brew =
  emoji = 🍺🍺
  install = downloaded-brew install {package}
  check = downloaded-brew list
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_sources_path(), downloaded_content).unwrap();

        // Create user custom that overrides
        let mut custom = SourcesDefinition::new();
        custom.insert(
            "brew".to_string(),
            crate::data::schemas::SourceDefinition {
                emoji: "🍻".to_string(),
                install: "user-brew install {package}".to_string(),
                check: "user-brew list".to_string(),
                prefix: None,
                overrides: None,
            },
        );

        let merged = manager.merge_sources(Some(&custom)).unwrap();
        let brew = merged.iter().find(|s| s.name == "brew").unwrap();

        // User custom should win
        assert_eq!(brew.origin, SourceOrigin::UserCustom);
        assert_eq!(brew.definition.emoji, "🍻");
    }

    #[test]
    fn test_clear_downloaded_sources() {
        let (manager, temp) = create_test_manager();

        // Create downloaded sources file
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_sources_path(), "test").unwrap();
        assert!(manager.has_downloaded_sources());

        manager.clear_downloaded_sources().unwrap();
        assert!(!manager.has_downloaded_sources());
    }

    #[test]
    fn test_sources_summary() {
        let (manager, temp) = create_test_manager();

        // Create downloaded sources
        let downloaded_content = r#"
new-from-download =
  emoji = 📥
  install = dl install {package}
  check = dl list
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_sources_path(), downloaded_content).unwrap();

        // Create user custom
        let mut custom = SourcesDefinition::new();
        custom.insert(
            "user-only".to_string(),
            crate::data::schemas::SourceDefinition {
                emoji: "👤".to_string(),
                install: "user install {package}".to_string(),
                check: "user list".to_string(),
                prefix: None,
                overrides: None,
            },
        );

        let merged = manager.merge_sources(Some(&custom)).unwrap();
        let summary = manager.sources_summary(&merged);

        assert!(summary.get(&SourceOrigin::Bundled).unwrap_or(&0) > &0);
        assert_eq!(summary.get(&SourceOrigin::Downloaded), Some(&1));
        assert_eq!(summary.get(&SourceOrigin::UserCustom), Some(&1));
    }

    #[test]
    fn test_source_origin_display() {
        assert_eq!(format!("{}", SourceOrigin::Bundled), "bundled");
        assert_eq!(format!("{}", SourceOrigin::Downloaded), "downloaded");
        assert_eq!(format!("{}", SourceOrigin::UserCustom), "custom");
    }
}

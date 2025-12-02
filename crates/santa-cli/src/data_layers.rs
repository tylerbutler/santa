//! Data layering system for Santa
//!
//! This module handles the three-layer configuration for both sources and packages:
//! 1. **Bundled** - Data compiled into the binary
//! 2. **Downloaded** - Data fetched from GitHub (stored locally)
//! 3. **User custom** - User-defined data in their config (highest priority)
//!
//! Items with the same name are completely overridden by higher-priority layers.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::data::schemas::{PackageDefinition, SourceDefinition, SourcesDefinition};

/// URL to fetch the latest sources from GitHub
pub const SOURCES_URL: &str =
    "https://raw.githubusercontent.com/tylerbutler/santa/main/crates/santa-cli/data/sources.ccl";

/// URL to fetch the latest packages from GitHub
pub const PACKAGES_URL: &str =
    "https://raw.githubusercontent.com/tylerbutler/santa/main/crates/santa-cli/data/known_packages.ccl";

/// Filename for downloaded sources
pub const DOWNLOADED_SOURCES_FILENAME: &str = "sources.ccl";

/// Filename for downloaded packages
pub const DOWNLOADED_PACKAGES_FILENAME: &str = "packages.ccl";

/// Type alias for packages definition
pub type PackagesDefinition = HashMap<String, PackageDefinition>;

/// Represents the origin of a data definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataOrigin {
    /// Compiled into the binary
    Bundled,
    /// Downloaded from GitHub
    Downloaded,
    /// User-defined in config
    UserCustom,
}

impl std::fmt::Display for DataOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataOrigin::Bundled => write!(f, "bundled"),
            DataOrigin::Downloaded => write!(f, "downloaded"),
            DataOrigin::UserCustom => write!(f, "custom"),
        }
    }
}

/// A source with its origin tracked
#[derive(Debug, Clone)]
pub struct LayeredSource {
    pub name: String,
    pub definition: SourceDefinition,
    pub origin: DataOrigin,
}

/// A package with its origin tracked
#[derive(Debug, Clone)]
pub struct LayeredPackage {
    pub name: String,
    pub definition: PackageDefinition,
    pub origin: DataOrigin,
}

/// Result of an update operation
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub sources_updated: bool,
    pub packages_updated: bool,
    pub sources_count: usize,
    pub packages_count: usize,
}

/// Manages the layered data system for both sources and packages
pub struct DataLayerManager {
    config_dir: PathBuf,
}

impl DataLayerManager {
    /// Create a new DataLayerManager with the given config directory
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// Create a DataLayerManager using the default config directory
    pub fn with_default_config_dir() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("com", "tylerbutler", "santa")
            .context("Failed to determine config directory")?
            .config_dir()
            .to_path_buf();
        Ok(Self::new(config_dir))
    }

    /// Path to the downloaded sources file
    pub fn downloaded_sources_path(&self) -> PathBuf {
        self.config_dir.join(DOWNLOADED_SOURCES_FILENAME)
    }

    /// Path to the downloaded packages file
    pub fn downloaded_packages_path(&self) -> PathBuf {
        self.config_dir.join(DOWNLOADED_PACKAGES_FILENAME)
    }

    // ============= Sources =============

    /// Fetch sources from GitHub and save to local storage
    pub fn update_sources(&self) -> Result<usize> {
        info!("Fetching sources from {}", SOURCES_URL);

        let response = ureq::get(SOURCES_URL)
            .call()
            .context("Failed to fetch sources from GitHub")?;

        let content = response
            .into_string()
            .context("Failed to read response body")?;

        // Validate that the content is valid CCL before saving
        let sources: SourcesDefinition =
            sickle::from_str(&content).context("Downloaded content is not valid CCL")?;

        let count = sources.len();

        // Ensure config directory exists
        fs::create_dir_all(&self.config_dir).context("Failed to create config directory")?;

        // Write the downloaded sources
        let path = self.downloaded_sources_path();
        fs::write(&path, &content)
            .with_context(|| format!("Failed to write sources to {:?}", path))?;

        info!("Sources updated successfully at {:?}", path);
        Ok(count)
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
                    origin: DataOrigin::Bundled,
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
                        origin: DataOrigin::Downloaded,
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
                        origin: DataOrigin::UserCustom,
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
    pub fn sources_summary(&self, sources: &[LayeredSource]) -> HashMap<DataOrigin, usize> {
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

    // ============= Packages =============

    /// Fetch packages from GitHub and save to local storage
    pub fn update_packages(&self) -> Result<usize> {
        info!("Fetching packages from {}", PACKAGES_URL);

        let response = ureq::get(PACKAGES_URL)
            .call()
            .context("Failed to fetch packages from GitHub")?;

        let content = response
            .into_string()
            .context("Failed to read response body")?;

        // Validate that the content is valid CCL before saving
        let packages: PackagesDefinition =
            sickle::from_str(&content).context("Downloaded packages content is not valid CCL")?;

        let count = packages.len();

        // Ensure config directory exists
        fs::create_dir_all(&self.config_dir).context("Failed to create config directory")?;

        // Write the downloaded packages
        let path = self.downloaded_packages_path();
        fs::write(&path, &content)
            .with_context(|| format!("Failed to write packages to {:?}", path))?;

        info!("Packages updated successfully at {:?}", path);
        Ok(count)
    }

    /// Load downloaded packages if they exist
    pub fn load_downloaded_packages(&self) -> Result<Option<PackagesDefinition>> {
        let path = self.downloaded_packages_path();
        if !path.exists() {
            debug!("No downloaded packages found at {:?}", path);
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read downloaded packages from {:?}", path))?;

        let packages: PackagesDefinition = sickle::from_str(&content)
            .with_context(|| format!("Failed to parse downloaded packages from {:?}", path))?;

        debug!("Loaded {} downloaded packages", packages.len());
        Ok(Some(packages))
    }

    /// Load bundled packages (compiled into binary)
    pub fn load_bundled_packages(&self) -> Result<PackagesDefinition> {
        let content = include_str!("../data/known_packages.ccl");
        let packages: PackagesDefinition =
            sickle::from_str(content).context("Failed to parse bundled packages")?;
        debug!("Loaded {} bundled packages", packages.len());
        Ok(packages)
    }

    /// Merge packages from all layers
    ///
    /// Priority (highest to lowest):
    /// 1. User custom packages (from config)
    /// 2. Downloaded packages
    /// 3. Bundled packages
    pub fn merge_packages(
        &self,
        user_custom: Option<&PackagesDefinition>,
    ) -> Result<Vec<LayeredPackage>> {
        let mut result: HashMap<String, LayeredPackage> = HashMap::new();

        // Layer 1: Bundled (lowest priority)
        let bundled = self.load_bundled_packages()?;
        for (name, definition) in bundled {
            result.insert(
                name.clone(),
                LayeredPackage {
                    name,
                    definition,
                    origin: DataOrigin::Bundled,
                },
            );
        }

        // Layer 2: Downloaded (overrides bundled)
        if let Some(downloaded) = self.load_downloaded_packages()? {
            for (name, definition) in downloaded {
                if result.contains_key(&name) {
                    debug!("Downloaded package '{}' overrides bundled", name);
                }
                result.insert(
                    name.clone(),
                    LayeredPackage {
                        name,
                        definition,
                        origin: DataOrigin::Downloaded,
                    },
                );
            }
        }

        // Layer 3: User custom (highest priority)
        if let Some(custom) = user_custom {
            for (name, definition) in custom {
                if result.contains_key(name) {
                    debug!("User custom package '{}' overrides lower layers", name);
                }
                result.insert(
                    name.clone(),
                    LayeredPackage {
                        name: name.clone(),
                        definition: definition.clone(),
                        origin: DataOrigin::UserCustom,
                    },
                );
            }
        }

        // Sort by name for consistent ordering
        let mut packages: Vec<LayeredPackage> = result.into_values().collect();
        packages.sort_by(|a, b| a.name.cmp(&b.name));

        info!("Merged {} total packages from all layers", packages.len());
        Ok(packages)
    }

    /// Get a summary of packages by origin
    pub fn packages_summary(&self, packages: &[LayeredPackage]) -> HashMap<DataOrigin, usize> {
        let mut summary = HashMap::new();
        for package in packages {
            *summary.entry(package.origin).or_insert(0) += 1;
        }
        summary
    }

    /// Check if downloaded packages exist
    pub fn has_downloaded_packages(&self) -> bool {
        self.downloaded_packages_path().exists()
    }

    /// Delete downloaded packages
    pub fn clear_downloaded_packages(&self) -> Result<()> {
        let path = self.downloaded_packages_path();
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("Failed to remove {:?}", path))?;
            info!("Removed downloaded packages at {:?}", path);
        }
        Ok(())
    }

    // ============= Combined Operations =============

    /// Update both sources and packages from GitHub
    pub fn update_all(&self) -> Result<UpdateResult> {
        let sources_count = self.update_sources()?;
        let packages_count = self.update_packages()?;

        Ok(UpdateResult {
            sources_updated: true,
            packages_updated: true,
            sources_count,
            packages_count,
        })
    }

    /// Clear all downloaded data
    pub fn clear_all(&self) -> Result<()> {
        self.clear_downloaded_sources()?;
        self.clear_downloaded_packages()?;
        Ok(())
    }

    /// Check if any downloaded data exists
    pub fn has_any_downloaded(&self) -> bool {
        self.has_downloaded_sources() || self.has_downloaded_packages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (DataLayerManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = DataLayerManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    // ============= Source Tests =============

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
    fn test_merge_sources_bundled_only() {
        let (manager, _temp) = create_test_manager();
        let merged = manager.merge_sources(None).unwrap();
        assert!(!merged.is_empty());

        // All should be bundled origin
        for source in &merged {
            assert_eq!(source.origin, DataOrigin::Bundled);
        }
    }

    #[test]
    fn test_merge_sources_with_user_custom_override() {
        let (manager, _temp) = create_test_manager();

        // Create a custom source that overrides brew
        let mut custom = SourcesDefinition::new();
        custom.insert(
            "brew".to_string(),
            SourceDefinition {
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
        assert_eq!(brew.origin, DataOrigin::UserCustom);
        assert_eq!(brew.definition.emoji, "🍻");
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
        assert_eq!(brew.origin, DataOrigin::Downloaded);
        assert_eq!(brew.definition.emoji, "🍺🍺");
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

    // ============= Package Tests =============

    #[test]
    fn test_load_bundled_packages() {
        let (manager, _temp) = create_test_manager();
        let packages = manager.load_bundled_packages().unwrap();
        assert!(!packages.is_empty(), "Should have bundled packages");
        assert!(packages.contains_key("bat"), "Should have bat package");
    }

    #[test]
    fn test_no_downloaded_packages_initially() {
        let (manager, _temp) = create_test_manager();
        assert!(!manager.has_downloaded_packages());
        let downloaded = manager.load_downloaded_packages().unwrap();
        assert!(downloaded.is_none());
    }

    #[test]
    fn test_merge_packages_bundled_only() {
        let (manager, _temp) = create_test_manager();
        let merged = manager.merge_packages(None).unwrap();
        assert!(!merged.is_empty());

        // All should be bundled origin
        for package in &merged {
            assert_eq!(package.origin, DataOrigin::Bundled);
        }
    }

    #[test]
    fn test_merge_packages_with_user_custom_override() {
        let (manager, _temp) = create_test_manager();

        // Create a custom package that overrides bat
        let mut custom = PackagesDefinition::new();
        custom.insert(
            "bat".to_string(),
            PackageDefinition::Simple(vec!["custom-source".to_string()]),
        );

        let merged = manager.merge_packages(Some(&custom)).unwrap();

        // Find bat in merged packages
        let bat = merged.iter().find(|p| p.name == "bat").unwrap();
        assert_eq!(bat.origin, DataOrigin::UserCustom);
    }

    #[test]
    fn test_merge_packages_with_new_user_package() {
        let (manager, _temp) = create_test_manager();

        // Create a completely new package
        let mut custom = PackagesDefinition::new();
        custom.insert(
            "my-custom-package".to_string(),
            PackageDefinition::Simple(vec!["brew".to_string(), "cargo".to_string()]),
        );

        let merged = manager.merge_packages(Some(&custom)).unwrap();

        // Should have the new package
        let custom_pkg = merged
            .iter()
            .find(|p| p.name == "my-custom-package")
            .unwrap();
        assert_eq!(custom_pkg.origin, DataOrigin::UserCustom);

        // Should still have bundled packages
        let bat = merged.iter().find(|p| p.name == "bat").unwrap();
        assert_eq!(bat.origin, DataOrigin::Bundled);
    }

    #[test]
    fn test_downloaded_packages_layer() {
        let (manager, temp) = create_test_manager();

        // Create downloaded packages file
        let downloaded_content = r#"
bat =
  = custom-source
  = another-source
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_packages_path(), downloaded_content).unwrap();

        assert!(manager.has_downloaded_packages());

        let merged = manager.merge_packages(None).unwrap();
        let bat = merged.iter().find(|p| p.name == "bat").unwrap();

        // Should be from downloaded layer
        assert_eq!(bat.origin, DataOrigin::Downloaded);
    }

    #[test]
    fn test_user_custom_overrides_downloaded_packages() {
        let (manager, temp) = create_test_manager();

        // Create downloaded packages file
        let downloaded_content = r#"
bat =
  = downloaded-source
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_packages_path(), downloaded_content).unwrap();

        // Create user custom that overrides
        let mut custom = PackagesDefinition::new();
        custom.insert(
            "bat".to_string(),
            PackageDefinition::Simple(vec!["user-source".to_string()]),
        );

        let merged = manager.merge_packages(Some(&custom)).unwrap();
        let bat = merged.iter().find(|p| p.name == "bat").unwrap();

        // User custom should win
        assert_eq!(bat.origin, DataOrigin::UserCustom);
    }

    #[test]
    fn test_clear_downloaded_packages() {
        let (manager, temp) = create_test_manager();

        // Create downloaded packages file
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_packages_path(), "test").unwrap();
        assert!(manager.has_downloaded_packages());

        manager.clear_downloaded_packages().unwrap();
        assert!(!manager.has_downloaded_packages());
    }

    // ============= Combined Tests =============

    #[test]
    fn test_clear_all() {
        let (manager, temp) = create_test_manager();

        // Create both downloaded files
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_sources_path(), "sources").unwrap();
        fs::write(manager.downloaded_packages_path(), "packages").unwrap();

        assert!(manager.has_any_downloaded());

        manager.clear_all().unwrap();

        assert!(!manager.has_downloaded_sources());
        assert!(!manager.has_downloaded_packages());
        assert!(!manager.has_any_downloaded());
    }

    #[test]
    fn test_data_origin_display() {
        assert_eq!(format!("{}", DataOrigin::Bundled), "bundled");
        assert_eq!(format!("{}", DataOrigin::Downloaded), "downloaded");
        assert_eq!(format!("{}", DataOrigin::UserCustom), "custom");
    }

    #[test]
    fn test_packages_summary() {
        let (manager, temp) = create_test_manager();

        // Create downloaded packages
        let downloaded_content = r#"
new-from-download =
  = some-source
"#;
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(manager.downloaded_packages_path(), downloaded_content).unwrap();

        // Create user custom
        let mut custom = PackagesDefinition::new();
        custom.insert(
            "user-only-pkg".to_string(),
            PackageDefinition::Simple(vec!["brew".to_string()]),
        );

        let merged = manager.merge_packages(Some(&custom)).unwrap();
        let summary = manager.packages_summary(&merged);

        assert!(summary.get(&DataOrigin::Bundled).unwrap_or(&0) > &0);
        assert_eq!(summary.get(&DataOrigin::Downloaded), Some(&1));
        assert_eq!(summary.get(&DataOrigin::UserCustom), Some(&1));
    }
}

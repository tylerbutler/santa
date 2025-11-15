use anyhow::{Context, Result};
use std::fs;
/// Migration utilities for transparently converting YAML configs to CCL
///
/// This module provides seamless migration from legacy YAML configuration files
/// to the new CCL format. When a user has an existing YAML config file,
/// it will be automatically converted to CCL format on first load.
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub mod yaml_to_hocon;

/// Configuration migration manager
pub struct ConfigMigrator {
    /// Whether to create backups of original files (default: true)
    pub create_backups: bool,
    /// Suffix for backup files (default: ".bak")
    pub backup_suffix: String,
    /// Whether this is a dry run (don't write files, just report what would happen)
    pub dry_run: bool,
}

impl Default for ConfigMigrator {
    fn default() -> Self {
        Self {
            create_backups: true,
            backup_suffix: ".bak".to_string(),
            dry_run: false,
        }
    }
}

impl ConfigMigrator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Transparently handle config file loading with automatic migration
    ///
    /// This function:
    /// 1. Checks if CCL version exists (.ccl)
    /// 2. If not, looks for YAML version (.yaml/.yml)
    /// 3. If YAML exists, converts it to CCL
    /// 4. Returns path to CCL file (existing or newly created)
    pub fn resolve_config_path(&self, requested_path: &Path) -> Result<PathBuf> {
        let requested_str = requested_path.to_string_lossy();

        // Try different extensions in order of preference
        let variants = self.generate_path_variants(requested_path);

        debug!("Resolving config path for: {}", requested_str);
        debug!("Checking variants: {:?}", variants);

        // First, check if CCL version already exists
        if let Some(ccl_path) = variants
            .iter()
            .find(|p| p.extension().is_some_and(|ext| ext == "ccl"))
        {
            if ccl_path.exists() {
                debug!("Found existing CCL config: {}", ccl_path.display());
                return Ok(ccl_path.clone());
            }
        }

        // Next, look for YAML versions to migrate
        for yaml_path in variants.iter().filter(|p| {
            p.extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        }) {
            if yaml_path.exists() {
                info!("Found YAML config file: {}", yaml_path.display());
                return self.migrate_yaml_to_hocon(yaml_path);
            }
        }

        // No existing config found - return the preferred CCL path
        let preferred_ccl = self.yaml_path_to_ccl_path(requested_path);
        debug!(
            "No existing config found, will use: {}",
            preferred_ccl.display()
        );
        Ok(preferred_ccl)
    }

    /// Generate all possible path variants for a config file
    fn generate_path_variants(&self, base_path: &Path) -> Vec<PathBuf> {
        let mut variants = Vec::new();

        // Start with the exact path provided
        variants.push(base_path.to_path_buf());

        // Try with different extensions
        if let Some(stem) = base_path.file_stem() {
            if let Some(parent) = base_path.parent() {
                variants.push(parent.join(format!("{}.ccl", stem.to_string_lossy())));
                variants.push(parent.join(format!("{}.yaml", stem.to_string_lossy())));
                variants.push(parent.join(format!("{}.yml", stem.to_string_lossy())));
            }
        }

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        variants.retain(|p| seen.insert(p.clone()));

        variants
    }

    /// Convert YAML config file to CCL format
    fn migrate_yaml_to_hocon(&self, yaml_path: &Path) -> Result<PathBuf> {
        let ccl_path = self.yaml_path_to_ccl_path(yaml_path);

        info!("Migrating {} → {}", yaml_path.display(), ccl_path.display());

        if self.dry_run {
            info!("[DRY RUN] Would migrate YAML config to CCL");
            return Ok(ccl_path);
        }

        // Read the YAML file
        let yaml_content = fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read YAML config: {}", yaml_path.display()))?;

        // Convert YAML to CCL
        let ccl_content = yaml_to_hocon::convert_yaml_to_hocon(&yaml_content)
            .with_context(|| "Failed to convert YAML content to CCL")?;

        // Ensure parent directory exists
        if let Some(parent) = ccl_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        // Write the CCL file
        fs::write(&ccl_path, ccl_content)
            .with_context(|| format!("Failed to write CCL config: {}", ccl_path.display()))?;

        info!("Successfully migrated to CCL: {}", ccl_path.display());

        // Create backup if requested
        if self.create_backups {
            self.backup_original_file(yaml_path)?;
        }

        // Always delete the original YAML file after successful migration
        self.remove_original_file(yaml_path)?;

        Ok(ccl_path)
    }

    /// Convert YAML file path to corresponding CCL path
    fn yaml_path_to_ccl_path(&self, yaml_path: &Path) -> PathBuf {
        if let Some(stem) = yaml_path.file_stem() {
            if let Some(parent) = yaml_path.parent() {
                return parent.join(format!("{}.ccl", stem.to_string_lossy()));
            }
        }

        // Fallback: just change extension
        yaml_path.with_extension("ccl")
    }

    /// Create backup of original YAML file
    fn backup_original_file(&self, original_path: &Path) -> Result<()> {
        let backup_path = original_path.with_extension(format!("yaml{}", self.backup_suffix));

        if backup_path.exists() {
            debug!("Backup already exists: {}", backup_path.display());
            return Ok(());
        }

        fs::copy(original_path, &backup_path).with_context(|| {
            format!(
                "Failed to create backup: {} → {}",
                original_path.display(),
                backup_path.display()
            )
        })?;

        info!("Created backup: {}", backup_path.display());
        Ok(())
    }

    /// Remove the original YAML file after successful migration
    fn remove_original_file(&self, original_path: &Path) -> Result<()> {
        if self.dry_run {
            info!(
                "[DRY RUN] Would remove original YAML file: {}",
                original_path.display()
            );
            return Ok(());
        }

        if !original_path.exists() {
            debug!("Original file already removed: {}", original_path.display());
            return Ok(());
        }

        fs::remove_file(original_path).with_context(|| {
            format!(
                "Failed to remove original YAML file: {}",
                original_path.display()
            )
        })?;

        info!("Removed original YAML file: {}", original_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_path_variants() {
        let migrator = ConfigMigrator::new();
        let base_path = Path::new("/home/user/.config/santa/config.yaml");

        let variants = migrator.generate_path_variants(base_path);

        // Should include the original path and variants with different extensions
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.yaml")));
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.ccl")));
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.yml")));
    }

    #[test]
    fn test_yaml_path_to_ccl_path() {
        let migrator = ConfigMigrator::new();

        let yaml_path = Path::new("/config/santa.yaml");
        let ccl_path = migrator.yaml_path_to_ccl_path(yaml_path);

        assert_eq!(ccl_path, PathBuf::from("/config/santa.ccl"));
    }

    #[test]
    fn test_resolve_config_path_ccl_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let ccl_file = temp_dir.path().join("config.ccl");
        fs::write(&ccl_file, "sources = [npm]\npackages = [git]")?;

        let migrator = ConfigMigrator::new();
        let requested = temp_dir.path().join("config.yaml");

        let resolved = migrator.resolve_config_path(&requested)?;

        // Should prefer existing CCL file
        assert_eq!(resolved, ccl_file);
        Ok(())
    }

    #[test]
    fn test_resolve_config_path_yaml_migration() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let yaml_file = temp_dir.path().join("config.yaml");
        fs::write(&yaml_file, "sources:\n  - npm\npackages:\n  - git")?;

        let migrator = ConfigMigrator::new();
        let resolved = migrator.resolve_config_path(&yaml_file)?;

        let expected_ccl = temp_dir.path().join("config.ccl");
        assert_eq!(resolved, expected_ccl);

        // Check that CCL file was created
        assert!(expected_ccl.exists());

        // Check that backup was created
        let backup_path = temp_dir.path().join("config.yaml.bak");
        assert!(backup_path.exists());

        // Check that original YAML file was deleted
        assert!(
            !yaml_file.exists(),
            "Original YAML file should be deleted after migration"
        );

        Ok(())
    }
}

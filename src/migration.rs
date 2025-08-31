/// Migration utilities for transparently converting YAML configs to HOCON
/// 
/// This module provides seamless migration from legacy YAML configuration files
/// to the new HOCON format. When a user has an existing YAML config file,
/// it will be automatically converted to HOCON format on first load.

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
use tracing::{info, warn, debug};

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
    /// 1. Checks if HOCON version exists (.conf)
    /// 2. If not, looks for YAML version (.yaml/.yml) 
    /// 3. If YAML exists, converts it to HOCON
    /// 4. Returns path to HOCON file (existing or newly created)
    pub fn resolve_config_path(&self, requested_path: &Path) -> Result<PathBuf> {
        let requested_str = requested_path.to_string_lossy();
        
        // Try different extensions in order of preference
        let variants = self.generate_path_variants(requested_path);
        
        debug!("Resolving config path for: {}", requested_str);
        debug!("Checking variants: {:?}", variants);
        
        // First, check if HOCON version already exists
        if let Some(hocon_path) = variants.iter().find(|p| p.extension().map_or(false, |ext| ext == "conf")) {
            if hocon_path.exists() {
                debug!("Found existing HOCON config: {}", hocon_path.display());
                return Ok(hocon_path.clone());
            }
        }
        
        // Next, look for YAML versions to migrate
        for yaml_path in variants.iter().filter(|p| {
            p.extension().map_or(false, |ext| ext == "yaml" || ext == "yml")
        }) {
            if yaml_path.exists() {
                info!("Found YAML config file: {}", yaml_path.display());
                return self.migrate_yaml_to_hocon(yaml_path);
            }
        }
        
        // No existing config found - return the preferred HOCON path
        let preferred_hocon = self.yaml_path_to_hocon_path(requested_path);
        debug!("No existing config found, will use: {}", preferred_hocon.display());
        Ok(preferred_hocon)
    }
    
    /// Generate all possible path variants for a config file
    fn generate_path_variants(&self, base_path: &Path) -> Vec<PathBuf> {
        let mut variants = Vec::new();
        
        // Start with the exact path provided
        variants.push(base_path.to_path_buf());
        
        // Try with different extensions
        if let Some(stem) = base_path.file_stem() {
            if let Some(parent) = base_path.parent() {
                variants.push(parent.join(format!("{}.conf", stem.to_string_lossy())));
                variants.push(parent.join(format!("{}.yaml", stem.to_string_lossy())));
                variants.push(parent.join(format!("{}.yml", stem.to_string_lossy())));
            }
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        variants.retain(|p| seen.insert(p.clone()));
        
        variants
    }
    
    /// Convert YAML config file to HOCON format
    fn migrate_yaml_to_hocon(&self, yaml_path: &Path) -> Result<PathBuf> {
        let hocon_path = self.yaml_path_to_hocon_path(yaml_path);
        
        info!("Migrating {} → {}", yaml_path.display(), hocon_path.display());
        
        if self.dry_run {
            info!("[DRY RUN] Would migrate YAML config to HOCON");
            return Ok(hocon_path);
        }
        
        // Read the YAML file
        let yaml_content = fs::read_to_string(yaml_path)
            .with_context(|| format!("Failed to read YAML config: {}", yaml_path.display()))?;
        
        // Convert YAML to HOCON
        let hocon_content = yaml_to_hocon::convert_yaml_to_hocon(&yaml_content)
            .with_context(|| "Failed to convert YAML content to HOCON")?;
        
        // Ensure parent directory exists
        if let Some(parent) = hocon_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        // Write the HOCON file
        fs::write(&hocon_path, hocon_content)
            .with_context(|| format!("Failed to write HOCON config: {}", hocon_path.display()))?;
        
        info!("Successfully migrated to HOCON: {}", hocon_path.display());
        
        // Create backup if requested
        if self.create_backups {
            self.backup_original_file(yaml_path)?;
        }
        
        // Always delete the original YAML file after successful migration
        self.remove_original_file(yaml_path)?;
        
        Ok(hocon_path)
    }
    
    /// Convert YAML file path to corresponding HOCON path  
    fn yaml_path_to_hocon_path(&self, yaml_path: &Path) -> PathBuf {
        if let Some(stem) = yaml_path.file_stem() {
            if let Some(parent) = yaml_path.parent() {
                return parent.join(format!("{}.conf", stem.to_string_lossy()));
            }
        }
        
        // Fallback: just change extension
        yaml_path.with_extension("conf")
    }
    
    /// Create backup of original YAML file
    fn backup_original_file(&self, original_path: &Path) -> Result<()> {
        let backup_path = original_path.with_extension(
            format!("yaml{}", self.backup_suffix)
        );
        
        if backup_path.exists() {
            debug!("Backup already exists: {}", backup_path.display());
            return Ok(());
        }
        
        fs::copy(original_path, &backup_path)
            .with_context(|| format!("Failed to create backup: {} → {}", 
                original_path.display(), backup_path.display()))?;
        
        info!("Created backup: {}", backup_path.display());
        Ok(())
    }
    
    /// Remove the original YAML file after successful migration
    fn remove_original_file(&self, original_path: &Path) -> Result<()> {
        if self.dry_run {
            info!("[DRY RUN] Would remove original YAML file: {}", original_path.display());
            return Ok(());
        }
        
        if !original_path.exists() {
            debug!("Original file already removed: {}", original_path.display());
            return Ok(());
        }
        
        fs::remove_file(original_path)
            .with_context(|| format!("Failed to remove original YAML file: {}", original_path.display()))?;
        
        info!("Removed original YAML file: {}", original_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{TempDir, NamedTempFile};
    use std::io::Write;
    
    #[test]
    fn test_generate_path_variants() {
        let migrator = ConfigMigrator::new();
        let base_path = Path::new("/home/user/.config/santa/config.yaml");
        
        let variants = migrator.generate_path_variants(base_path);
        
        // Should include the original path and variants with different extensions
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.yaml")));
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.conf")));
        assert!(variants.contains(&PathBuf::from("/home/user/.config/santa/config.yml")));
    }
    
    #[test]  
    fn test_yaml_path_to_hocon_path() {
        let migrator = ConfigMigrator::new();
        
        let yaml_path = Path::new("/config/santa.yaml");
        let hocon_path = migrator.yaml_path_to_hocon_path(yaml_path);
        
        assert_eq!(hocon_path, PathBuf::from("/config/santa.conf"));
    }
    
    #[test]
    fn test_resolve_config_path_hocon_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let hocon_file = temp_dir.path().join("config.conf");
        fs::write(&hocon_file, "sources = [npm]\npackages = [git]")?;
        
        let migrator = ConfigMigrator::new();
        let requested = temp_dir.path().join("config.yaml");
        
        let resolved = migrator.resolve_config_path(&requested)?;
        
        // Should prefer existing HOCON file
        assert_eq!(resolved, hocon_file);
        Ok(())
    }
    
    #[test]
    fn test_resolve_config_path_yaml_migration() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let yaml_file = temp_dir.path().join("config.yaml");
        fs::write(&yaml_file, "sources:\n  - npm\npackages:\n  - git")?;
        
        let migrator = ConfigMigrator::new();
        let resolved = migrator.resolve_config_path(&yaml_file)?;
        
        let expected_hocon = temp_dir.path().join("config.conf");
        assert_eq!(resolved, expected_hocon);
        
        // Check that HOCON file was created
        assert!(expected_hocon.exists());
        
        // Check that backup was created
        let backup_path = temp_dir.path().join("config.yaml.bak");
        assert!(backup_path.exists());
        
        // Check that original YAML file was deleted
        assert!(!yaml_file.exists(), "Original YAML file should be deleted after migration");
        
        Ok(())
    }
}
//! Shared catalog types and utilities for dev-tools binaries.
//!
//! This module provides unified types for reading and writing:
//! - `packages.ccl` - Package catalog with metadata and verification status
//! - `sources/*.ccl` - Per-source package definitions

use anyhow::{Context, Result};
use sickle::printer::CclPrinter;
use sickle::CclObject;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Package metadata from the catalog (packages.ccl)
#[derive(Debug, Clone, Default)]
pub struct CatalogEntry {
    /// Package description
    pub description: Option<String>,
    /// Package homepage URL
    pub homepage: Option<String>,
    /// Whether this package has been verified against Repology
    pub verified: bool,
    /// Verification date (YYYY-MM-DD format)
    pub verified_date: Option<String>,
}

/// Package entry from a source CCL file (e.g., brew.ccl)
#[derive(Debug, Clone)]
pub struct SourceEntry {
    /// The package name as it appears in the source (e.g., "gh" in brew)
    pub source_name: String,
    /// The canonical name it maps to (e.g., "github-cli"), or same as source_name
    pub canonical_name: String,
    /// Which source file this came from (e.g., "brew", "apt")
    pub source: String,
    /// Additional configuration (pre-install hooks, etc.)
    pub config: BTreeMap<String, String>,
}

impl SourceEntry {
    /// Create a new source entry with the same source and canonical name
    pub fn simple(name: String, source: String) -> Self {
        Self {
            source_name: name.clone(),
            canonical_name: name,
            source,
            config: BTreeMap::new(),
        }
    }

    /// Create a new source entry with a name override
    pub fn with_override(source_name: String, canonical_name: String, source: String) -> Self {
        Self {
            source_name,
            canonical_name,
            source,
            config: BTreeMap::new(),
        }
    }
}

/// Extract a string value from a CCL object.
///
/// Returns Some(string) if the object contains exactly one key with an empty value,
/// which is how CCL represents simple string values like `key = value`.
pub fn extract_string_value(obj: &CclObject) -> Option<String> {
    if obj.len() == 1 && obj.values().next().unwrap().is_empty() {
        Some(obj.keys().next().unwrap().clone())
    } else {
        None
    }
}

/// Load the package catalog from packages.ccl
pub fn load_catalog(path: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    let mut catalog = BTreeMap::new();

    if !path.exists() {
        return Ok(catalog);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read catalog: {}", path.display()))?;

    let model = sickle::load(&content)
        .with_context(|| format!("Failed to parse catalog: {}", path.display()))?;

    for key in model.keys() {
        // Skip comments and empty keys
        if key.starts_with('/') || key.is_empty() {
            continue;
        }

        let value = model.get(key)?;
        let mut entry = CatalogEntry::default();

        if !value.is_empty() {
            if let Ok(desc_obj) = value.get("description") {
                if let Some(desc) = extract_string_value(desc_obj) {
                    entry.description = Some(desc);
                }
            }
            if let Ok(homepage_obj) = value.get("homepage") {
                if let Some(homepage) = extract_string_value(homepage_obj) {
                    entry.homepage = Some(homepage);
                }
            }
            // Check for verified field
            if let Ok(verified_obj) = value.get("verified") {
                entry.verified = true;
                if let Some(date) = extract_string_value(verified_obj) {
                    entry.verified_date = Some(date);
                }
            }
        }

        catalog.insert(key.clone(), entry);
    }

    Ok(catalog)
}

/// Save the package catalog to packages.ccl
pub fn save_catalog(path: &Path, catalog: &BTreeMap<String, CatalogEntry>) -> Result<()> {
    let mut obj = CclObject::new();
    obj.add_comment("Core package catalog");
    obj.add_comment("Source of truth for package identity and metadata");
    obj.add_blank_line();

    let map = obj.inner_mut();

    for (name, entry) in catalog {
        // Skip entries with no metadata at all
        if entry.description.is_none() && entry.homepage.is_none() && !entry.verified {
            continue;
        }

        let mut pkg_obj = CclObject::new();
        let pkg_map = pkg_obj.inner_mut();

        if let Some(ref desc) = entry.description {
            pkg_map.insert(
                "description".to_string(),
                vec![CclObject::from_string(desc)],
            );
        }

        if let Some(ref homepage) = entry.homepage {
            pkg_map.insert(
                "homepage".to_string(),
                vec![CclObject::from_string(homepage)],
            );
        }

        if entry.verified {
            let date = entry
                .verified_date
                .clone()
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
            pkg_map.insert("verified".to_string(), vec![CclObject::from_string(&date)]);
        }

        map.insert(name.clone(), vec![pkg_obj]);
    }

    let printer = CclPrinter::new();
    let output = printer.print(&obj);
    fs::write(path, output).with_context(|| format!("Failed to write: {}", path.display()))?;

    Ok(())
}

/// Get set of verified package names from the catalog
pub fn get_verified_packages(catalog_path: &Path) -> Result<BTreeSet<String>> {
    let catalog = load_catalog(catalog_path)?;
    Ok(catalog
        .into_iter()
        .filter(|(_, entry)| entry.verified)
        .map(|(name, _)| name)
        .collect())
}

/// Read all packages from a source CCL file
pub fn read_source_file(path: &Path, source_name: &str) -> Result<Vec<SourceEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read source file: {}", path.display()))?;

    let model = sickle::load(&content)
        .with_context(|| format!("Failed to parse source file: {}", path.display()))?;

    let mut entries = Vec::new();

    for key in model.keys() {
        // Skip comments and empty keys
        if key.starts_with('/') || key.is_empty() {
            continue;
        }

        let value = model.get(key)?;

        // Determine canonical name and config
        let (canonical, config) = if value.is_empty() {
            // Empty value means same name: `bat =`
            (key.clone(), BTreeMap::new())
        } else if let Some(s) = extract_string_value(value) {
            if s.is_empty() {
                (key.clone(), BTreeMap::new())
            } else {
                // Name override: `gh = github-cli`
                (s, BTreeMap::new())
            }
        } else {
            // Nested object with config
            let mut config = BTreeMap::new();
            let mut canonical = key.clone();

            for nested_key in value.keys() {
                if let Ok(nested_value) = value.get(nested_key) {
                    if let Some(s) = extract_string_value(nested_value) {
                        // Special handling for description - skip it for source entries
                        if nested_key != "_description" {
                            config.insert(nested_key.clone(), s);
                        }
                    }
                }
            }

            // Check if there's a canonical name override in config
            if let Some(name) = config.remove("_canonical") {
                canonical = name;
            }

            (canonical, config)
        };

        entries.push(SourceEntry {
            source_name: key.clone(),
            canonical_name: canonical,
            source: source_name.to_string(),
            config,
        });
    }

    Ok(entries)
}

/// Get all source file names from the sources directory
pub fn get_all_source_names(sources_dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    for entry in fs::read_dir(sources_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("ccl") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }

    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_entry_default() {
        let entry = CatalogEntry::default();
        assert!(entry.description.is_none());
        assert!(entry.homepage.is_none());
        assert!(!entry.verified);
        assert!(entry.verified_date.is_none());
    }

    #[test]
    fn test_source_entry_simple() {
        let entry = SourceEntry::simple("bat".to_string(), "brew".to_string());
        assert_eq!(entry.source_name, "bat");
        assert_eq!(entry.canonical_name, "bat");
        assert_eq!(entry.source, "brew");
        assert!(entry.config.is_empty());
    }

    #[test]
    fn test_source_entry_with_override() {
        let entry = SourceEntry::with_override(
            "gh".to_string(),
            "github-cli".to_string(),
            "brew".to_string(),
        );
        assert_eq!(entry.source_name, "gh");
        assert_eq!(entry.canonical_name, "github-cli");
        assert_eq!(entry.source, "brew");
    }

    #[test]
    fn test_extract_string_value() {
        let model = sickle::load("test = value").unwrap();
        let value = model.get("test").unwrap();
        assert_eq!(extract_string_value(value), Some("value".to_string()));
    }

    #[test]
    fn test_extract_string_value_nested_returns_none() {
        let model = sickle::load("test =\n  nested = value").unwrap();
        let value = model.get("test").unwrap();
        assert_eq!(extract_string_value(value), None);
    }
}

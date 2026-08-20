//! Shared catalog types and utilities for dev-tools binaries.
//!
//! This module provides unified types for reading and writing:
//! - `packages.ccl` - Package catalog with metadata and verification status
//! - `sources/*.ccl` - Per-source package definitions

use anyhow::{Context, Result};
use sickle::{table, value, DocumentMut, Item};
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

/// Extract a scalar value from a CCL node.
///
/// Returns `Some(text)` for a scalar such as `key = value`, and `None` for a
/// nested block or a list.
pub fn extract_string_value(item: &Item) -> Option<String> {
    item.as_str().map(str::to_string)
}

/// Load the package catalog from packages.ccl
pub fn load_catalog(path: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    let mut catalog = BTreeMap::new();

    if !path.exists() {
        return Ok(catalog);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read catalog: {}", path.display()))?;

    let document = DocumentMut::parse(&content)
        .with_context(|| format!("Failed to parse catalog: {}", path.display()))?;

    for key in document.as_table().unique_keys() {
        // Skip comment-shaped keys and bare list items
        if key.starts_with('/') || key.is_empty() {
            continue;
        }

        let mut entry = CatalogEntry::default();

        if let Some(fields) = document.as_table().get(key).and_then(Item::as_table) {
            entry.description = fields.get("description").and_then(extract_string_value);
            entry.homepage = fields.get("homepage").and_then(extract_string_value);
            if let Some(verified) = fields.get("verified") {
                entry.verified = true;
                entry.verified_date = extract_string_value(verified);
            }
        }

        catalog.insert(key.to_string(), entry);
    }

    Ok(catalog)
}

/// Save the package catalog to packages.ccl
pub fn save_catalog(path: &Path, catalog: &BTreeMap<String, CatalogEntry>) -> Result<()> {
    let mut document = DocumentMut::new();

    for (name, entry) in catalog {
        // Skip entries with no metadata at all
        if entry.description.is_none() && entry.homepage.is_none() && !entry.verified {
            continue;
        }

        let mut package = table();
        let fields = package.as_table_mut().expect("just created a table");

        if let Some(ref desc) = entry.description {
            fields.append("description", value(desc));
        }

        if let Some(ref homepage) = entry.homepage {
            fields.append("homepage", value(homepage));
        }

        if entry.verified {
            let date = entry
                .verified_date
                .clone()
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
            fields.append("verified", value(date));
        }

        document.as_table_mut().append(name.as_str(), package);
    }

    // Header comments read top-down, so insert them in order above the first entry.
    if !document.is_empty() {
        document.insert_comment_at(0, "Core package catalog")?;
        document.insert_comment_at(0, "Source of truth for package identity and metadata")?;
        document.insert_blank_line_at(0)?;
    }

    fs::write(path, document.to_string())
        .with_context(|| format!("Failed to write: {}", path.display()))?;

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

    let document = DocumentMut::parse(&content)
        .with_context(|| format!("Failed to parse source file: {}", path.display()))?;

    let mut entries = Vec::new();

    for key in document.as_table().unique_keys() {
        // Skip comment-shaped keys and bare list items
        if key.starts_with('/') || key.is_empty() {
            continue;
        }

        let item = document
            .as_table()
            .get(key)
            .expect("key came from the table");

        // Determine canonical name and config
        let (canonical, config) = match item {
            // `bat =` keeps its own name; `gh = github-cli` overrides it.
            Item::Value(scalar) if scalar.as_str().is_empty() => (key.to_string(), BTreeMap::new()),
            Item::Value(scalar) => (scalar.as_str().to_string(), BTreeMap::new()),
            Item::Table(fields) => {
                let mut config = BTreeMap::new();
                for field in fields.unique_keys() {
                    // `_description` is catalog metadata, not source config
                    if field == "_description" {
                        continue;
                    }
                    if let Some(text) = fields.get(field).and_then(extract_string_value) {
                        config.insert(field.to_string(), text);
                    }
                }
                let canonical = config
                    .remove("_canonical")
                    .unwrap_or_else(|| key.to_string());
                (canonical, config)
            }
            _ => (key.to_string(), BTreeMap::new()),
        };

        entries.push(SourceEntry {
            source_name: key.to_string(),
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
    use tempfile::TempDir;

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
        let doc = DocumentMut::parse("test = value").unwrap();
        let item = doc.as_table().get("test").unwrap();
        assert_eq!(extract_string_value(item), Some("value".to_string()));
    }

    #[test]
    fn test_extract_string_value_nested_returns_none() {
        let doc = DocumentMut::parse("test =\n  nested = value").unwrap();
        let item = doc.as_table().get("test").unwrap();
        assert_eq!(extract_string_value(item), None);
    }

    #[test]
    fn test_extract_string_value_empty_object() {
        assert_eq!(extract_string_value(&table()), None);
    }

    #[test]
    fn test_load_catalog_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.ccl");
        let catalog = load_catalog(&path).unwrap();
        assert!(catalog.is_empty());
    }

    #[test]
    fn test_load_catalog_with_metadata() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("packages.ccl");
        let content = r#"
bat =
  description = A cat clone with wings
  homepage = https://github.com/sharkdp/bat
  verified = 2024-01-15
ripgrep =
  description = Fast grep
"#;
        fs::write(&path, content).unwrap();

        let catalog = load_catalog(&path).unwrap();
        assert_eq!(catalog.len(), 2);

        let bat = catalog.get("bat").unwrap();
        assert_eq!(bat.description, Some("A cat clone with wings".to_string()));
        assert_eq!(
            bat.homepage,
            Some("https://github.com/sharkdp/bat".to_string())
        );
        assert!(bat.verified);
        assert_eq!(bat.verified_date, Some("2024-01-15".to_string()));

        let rg = catalog.get("ripgrep").unwrap();
        assert_eq!(rg.description, Some("Fast grep".to_string()));
        assert!(!rg.verified);
    }

    #[test]
    fn test_load_catalog_skips_comments() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("packages.ccl");
        let content = r#"
/= This is a comment
bat =
  description = A cat clone
"#;
        fs::write(&path, content).unwrap();

        let catalog = load_catalog(&path).unwrap();
        assert_eq!(catalog.len(), 1);
        assert!(catalog.contains_key("bat"));
        assert!(!catalog.contains_key("/= This is a comment"));
    }

    #[test]
    fn test_save_and_load_catalog_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("packages.ccl");

        let mut catalog = BTreeMap::new();
        catalog.insert(
            "bat".to_string(),
            CatalogEntry {
                description: Some("A cat clone".to_string()),
                homepage: Some("https://example.com".to_string()),
                verified: true,
                verified_date: Some("2024-06-01".to_string()),
            },
        );
        catalog.insert(
            "ripgrep".to_string(),
            CatalogEntry {
                description: Some("Fast grep".to_string()),
                homepage: None,
                verified: false,
                verified_date: None,
            },
        );

        save_catalog(&path, &catalog).unwrap();
        let loaded = load_catalog(&path).unwrap();

        // bat should round-trip fully
        let bat = loaded.get("bat").unwrap();
        assert_eq!(bat.description, Some("A cat clone".to_string()));
        assert_eq!(bat.homepage, Some("https://example.com".to_string()));
        assert!(bat.verified);
        assert_eq!(bat.verified_date, Some("2024-06-01".to_string()));

        // ripgrep should round-trip (description only)
        let rg = loaded.get("ripgrep").unwrap();
        assert_eq!(rg.description, Some("Fast grep".to_string()));
        assert!(!rg.verified);
    }

    #[test]
    fn test_save_catalog_skips_empty_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("packages.ccl");

        let mut catalog = BTreeMap::new();
        catalog.insert("bat".to_string(), CatalogEntry::default());
        catalog.insert(
            "ripgrep".to_string(),
            CatalogEntry {
                description: Some("Fast grep".to_string()),
                ..Default::default()
            },
        );

        save_catalog(&path, &catalog).unwrap();
        let loaded = load_catalog(&path).unwrap();

        // Empty entry should be skipped
        assert!(!loaded.contains_key("bat"));
        assert!(loaded.contains_key("ripgrep"));
    }

    #[test]
    fn test_get_verified_packages() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("packages.ccl");
        let content = r#"
bat =
  verified = 2024-01-01
ripgrep =
  description = Fast grep
fd =
  verified = 2024-02-01
"#;
        fs::write(&path, content).unwrap();

        let verified = get_verified_packages(&path).unwrap();
        assert_eq!(verified.len(), 2);
        assert!(verified.contains("bat"));
        assert!(verified.contains("fd"));
        assert!(!verified.contains("ripgrep"));
    }

    #[test]
    fn test_read_source_file_simple_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("brew.ccl");
        let content = r#"
bat =
ripgrep =
"#;
        fs::write(&path, content).unwrap();

        let entries = read_source_file(&path, "brew").unwrap();
        assert_eq!(entries.len(), 2);

        let bat = entries.iter().find(|e| e.source_name == "bat").unwrap();
        assert_eq!(bat.canonical_name, "bat");
        assert_eq!(bat.source, "brew");

        let rg = entries.iter().find(|e| e.source_name == "ripgrep").unwrap();
        assert_eq!(rg.canonical_name, "ripgrep");
    }

    #[test]
    fn test_read_source_file_with_name_override() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("brew.ccl");
        let content = "gh = github-cli\n";
        fs::write(&path, content).unwrap();

        let entries = read_source_file(&path, "brew").unwrap();
        assert_eq!(entries.len(), 1);

        let gh = &entries[0];
        assert_eq!(gh.source_name, "gh");
        assert_eq!(gh.canonical_name, "github-cli");
    }

    #[test]
    fn test_read_source_file_with_nested_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("apt.ccl");
        let content = r#"
python3-pip =
  _canonical = pip
  pre_install = apt update
"#;
        fs::write(&path, content).unwrap();

        let entries = read_source_file(&path, "apt").unwrap();
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.source_name, "python3-pip");
        assert_eq!(entry.canonical_name, "pip");
        assert_eq!(
            entry.config.get("pre_install"),
            Some(&"apt update".to_string())
        );
        // _canonical should be removed from config
        assert!(!entry.config.contains_key("_canonical"));
    }

    #[test]
    fn test_read_source_file_skips_description() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("brew.ccl");
        let content = r#"
bat =
  _description = A cat clone
  tap = sharkdp/bat
"#;
        fs::write(&path, content).unwrap();

        let entries = read_source_file(&path, "brew").unwrap();
        let entry = &entries[0];
        // _description should not be in config
        assert!(!entry.config.contains_key("_description"));
        assert_eq!(entry.config.get("tap"), Some(&"sharkdp/bat".to_string()));
    }

    #[test]
    fn test_get_all_source_names() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("brew.ccl"), "bat =\n").unwrap();
        fs::write(dir.path().join("apt.ccl"), "bat =\n").unwrap();
        fs::write(dir.path().join("cargo.ccl"), "bat =\n").unwrap();
        fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();

        let names = get_all_source_names(dir.path()).unwrap();
        assert_eq!(names, vec!["apt", "brew", "cargo"]);
    }

    #[test]
    fn test_get_all_source_names_empty_dir() {
        let dir = TempDir::new().unwrap();
        let names = get_all_source_names(dir.path()).unwrap();
        assert!(names.is_empty());
    }
}

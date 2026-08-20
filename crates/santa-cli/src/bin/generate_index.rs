//! Generate package_index.ccl from source-organized package files.
//!
//! This binary reads all CCL files in data/sources/ and generates a unified
//! package index that maps package names to their available sources.
//! It also reads packages.ccl catalog for descriptions and other metadata.
//!
//! By default, only verified packages are included in the output.
//! Use --include-unverified to include all packages.

use anyhow::{Context, Result};
use clap::Parser;
use santa::catalog::{self, CatalogEntry};
use sickle::{table, value, DocumentMut, Item};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Generate unified package index from source files
#[derive(Parser)]
#[command(name = "generate-index")]
#[command(about = "Generate known_packages.ccl from source files")]
struct Cli {
    /// Include packages that haven't been verified against Repology
    #[arg(long)]
    include_unverified: bool,
}

/// Configuration for a package from a specific source
#[derive(Debug, Clone)]
enum PackageConfig {
    /// Simple package with no config (just available from this source)
    Simple,
    /// Package with a name override (e.g., "rg" instead of "ripgrep")
    NameOverride(String),
    /// Package with complex config (pre-install hooks, install_suffix, etc.)
    Complex(BTreeMap<String, String>),
}

/// Aggregated package data across all sources
#[derive(Debug)]
struct PackageData {
    /// Map of source name to package config
    sources: BTreeMap<String, PackageConfig>,
    /// Package description (if provided)
    description: Option<String>,
}

impl PackageData {
    fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
            description: None,
        }
    }

    fn add_source(&mut self, source: String, config: PackageConfig) {
        self.sources.insert(source, config);
    }

    fn set_description(&mut self, desc: String) {
        // Only set if not already set (first one wins)
        if self.description.is_none() && !desc.is_empty() {
            self.description = Some(desc);
        }
    }

    /// Returns true if this is a simple package (all sources have no config, no description)
    fn is_simple(&self) -> bool {
        self.description.is_none()
            && self
                .sources
                .values()
                .all(|config| matches!(config, PackageConfig::Simple))
    }
}

/// Re-export for local use
fn extract_string_value(item: &Item) -> Option<String> {
    catalog::extract_string_value(item)
}

/// Parsed package info from a source file
#[derive(Debug)]
struct ParsedPackage {
    config: PackageConfig,
    description: Option<String>,
}

fn parse_source_file(path: &Path) -> Result<BTreeMap<String, ParsedPackage>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read source file: {}", path.display()))?;

    let document = DocumentMut::parse(&content)
        .with_context(|| format!("Failed to parse CCL in: {}", path.display()))?;

    let mut packages = BTreeMap::new();

    for key in document.as_table().unique_keys() {
        // Skip comment-shaped keys
        if key.starts_with('/') {
            continue;
        }

        let item = document
            .as_table()
            .get(key)
            .expect("key came from the table");

        let (config, description) = match item {
            // `bat =` is a plain availability marker; `gh = github-cli` renames.
            Item::Value(scalar) if scalar.as_str().is_empty() => (PackageConfig::Simple, None),
            Item::Value(scalar) => (
                PackageConfig::NameOverride(scalar.as_str().to_string()),
                None,
            ),
            Item::Table(fields) => {
                let mut config_map = BTreeMap::new();
                let mut desc = None;

                for field in fields.unique_keys() {
                    let Some(text) = fields.get(field).and_then(extract_string_value) else {
                        continue;
                    };
                    if field == "_description" {
                        desc = Some(text);
                    } else {
                        config_map.insert(field.to_string(), text);
                    }
                }

                let config = if config_map.is_empty() {
                    PackageConfig::Simple
                } else {
                    PackageConfig::Complex(config_map)
                };
                (config, desc)
            }
            _ => (PackageConfig::Simple, None),
        };

        packages.insert(
            key.to_string(),
            ParsedPackage {
                config,
                description,
            },
        );
    }

    Ok(packages)
}

fn generate_index(
    sources_dir: &Path,
    catalog: &BTreeMap<String, CatalogEntry>,
    include_unverified: bool,
) -> Result<(String, usize, usize)> {
    let mut all_packages: BTreeMap<String, PackageData> = BTreeMap::new();
    let mut document = DocumentMut::new();

    // Read all source files
    for entry in fs::read_dir(sources_dir).with_context(|| {
        format!(
            "Failed to read sources directory: {}",
            sources_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("ccl") {
            let source_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Invalid source file name")?
                .to_string();

            let packages = parse_source_file(&path)?;

            for (package_name, parsed) in packages {
                let pkg_data = all_packages
                    .entry(package_name.clone())
                    .or_insert_with(PackageData::new);
                pkg_data.add_source(source_name.clone(), parsed.config);

                // Use source description if provided
                if let Some(desc) = parsed.description {
                    pkg_data.set_description(desc);
                }

                // Fall back to catalog description if no source description
                if pkg_data.description.is_none() {
                    if let Some(catalog_entry) = catalog.get(&package_name) {
                        if let Some(ref desc) = catalog_entry.description {
                            pkg_data.set_description(desc.clone());
                        }
                    }
                }
            }
        }
    }

    let total_packages = all_packages.len();

    // Filter to only verified packages unless include_unverified is set
    let filtered_packages: BTreeMap<String, PackageData> = if include_unverified {
        all_packages
    } else {
        all_packages
            .into_iter()
            .filter(|(name, _)| {
                catalog
                    .get(name)
                    .map(|entry| entry.verified)
                    .unwrap_or(false)
            })
            .collect()
    };

    let included_packages = filtered_packages.len();

    // Build the document, then attach the header and section comments as
    // trivia above the entries they introduce.
    let mut simple_packages = Vec::new();
    let mut complex_packages = Vec::new();

    for (name, data) in &filtered_packages {
        if data.is_simple() {
            simple_packages.push(name);
        } else {
            complex_packages.push(name);
        }
    }

    for package_name in simple_packages {
        let data = &filtered_packages[package_name];
        let sources: Vec<&str> = data.sources.keys().map(String::as_str).collect();
        document
            .as_table_mut()
            .append(package_name.as_str(), sources);
    }

    let first_complex = complex_packages.first().map(|name| (*name).clone());

    for package_name in complex_packages {
        let data = &filtered_packages[package_name];
        let mut package = table();
        let fields = package.as_table_mut().expect("just created a table");

        if let Some(desc) = &data.description {
            fields.append("_description", value(desc));
        }

        let mut simple_sources = Vec::new();
        for (source, config) in &data.sources {
            match config {
                PackageConfig::Simple => simple_sources.push(source.as_str()),
                PackageConfig::NameOverride(name) => fields.append(source.as_str(), value(name)),
                PackageConfig::Complex(config_map) => {
                    let mut nested = table();
                    let nested_fields = nested.as_table_mut().expect("just created a table");
                    for (key, val) in config_map {
                        nested_fields.append(key.as_str(), value(val));
                    }
                    fields.append(source.as_str(), nested);
                }
            }
        }

        if !simple_sources.is_empty() {
            fields.append("_sources", simple_sources);
        }

        document
            .as_table_mut()
            .append(package_name.as_str(), package);
    }

    // `None` marks a blank line. Header trivia attaches above the first entry,
    // or becomes the whole document when there are no packages at all.
    let mut header = vec![
        Some("Generated package index"),
        Some("DO NOT EDIT - Generated from data/sources/*.ccl"),
        Some("Run: just generate-index to regenerate"),
    ];
    if !include_unverified {
        header.push(Some("Only verified packages are included"));
    }
    header.push(None);
    header.push(Some(
        "Packages with simple format (no source-specific overrides)",
    ));

    for line in header {
        match (line, document.is_empty()) {
            (Some(text), false) => document.insert_comment_at(0, text)?,
            (Some(text), true) => document.push_comment(text)?,
            (None, false) => document.insert_blank_line_at(0)?,
            (None, true) => document.push_blank_line(),
        }
    }

    if let Some(name) = first_complex {
        document.insert_blank_line_before([name.as_str()])?;
        document.insert_comment_before(
            [name.as_str()],
            "Packages with complex format (have source-specific overrides or descriptions)",
        )?;
    }

    Ok((document.to_string(), total_packages, included_packages))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sources_dir = manifest_dir.join("data").join("sources");
    let catalog_file = manifest_dir.join("data").join("packages.ccl");
    let output_file = manifest_dir.join("data").join("known_packages.ccl");

    if !sources_dir.exists() {
        anyhow::bail!("Sources directory not found: {}", sources_dir.display());
    }

    // Load the package catalog for metadata
    println!("Loading package catalog from: {}", catalog_file.display());
    let catalog = catalog::load_catalog(&catalog_file)?;
    println!("Loaded {} packages from catalog", catalog.len());

    println!("Reading source files from: {}", sources_dir.display());

    // Generate index with catalog metadata
    let (index_content, total, included) =
        generate_index(&sources_dir, &catalog, cli.include_unverified)?;

    // Write output
    fs::write(&output_file, &index_content).context("Failed to write output file")?;

    println!("Generated package index: {}", output_file.display());
    if cli.include_unverified {
        println!("Included all {} packages", total);
    } else {
        println!(
            "Included {} verified packages (excluded {} unverified)",
            included,
            total - included
        );
    }

    Ok(())
}

/// Parse CCL content directly (for testing without file I/O)
#[cfg(test)]
fn parse_source_content(content: &str) -> Result<BTreeMap<String, ParsedPackage>> {
    let document =
        DocumentMut::parse(content).with_context(|| "Failed to parse CCL content".to_string())?;

    let mut packages = BTreeMap::new();

    for key in document.as_table().unique_keys() {
        // Skip comment-shaped keys
        if key.starts_with('/') {
            continue;
        }

        let item = document
            .as_table()
            .get(key)
            .expect("key came from the table");

        let (config, description) = match item {
            // `bat =` is a plain availability marker; `gh = github-cli` renames.
            Item::Value(scalar) if scalar.as_str().is_empty() => (PackageConfig::Simple, None),
            Item::Value(scalar) => (
                PackageConfig::NameOverride(scalar.as_str().to_string()),
                None,
            ),
            Item::Table(fields) => {
                let mut config_map = BTreeMap::new();
                let mut desc = None;

                for field in fields.unique_keys() {
                    let Some(text) = fields.get(field).and_then(extract_string_value) else {
                        continue;
                    };
                    if field == "_description" {
                        desc = Some(text);
                    } else {
                        config_map.insert(field.to_string(), text);
                    }
                }

                let config = if config_map.is_empty() {
                    PackageConfig::Simple
                } else {
                    PackageConfig::Complex(config_map)
                };
                (config, desc)
            }
            _ => (PackageConfig::Simple, None),
        };

        packages.insert(
            key.to_string(),
            ParsedPackage {
                config,
                description,
            },
        );
    }

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_data_new() {
        let data = PackageData::new();
        assert!(data.sources.is_empty());
        assert!(data.description.is_none());
    }

    #[test]
    fn test_package_data_add_source() {
        let mut data = PackageData::new();
        data.add_source("brew".to_string(), PackageConfig::Simple);
        assert_eq!(data.sources.len(), 1);
        assert!(data.sources.contains_key("brew"));
    }

    #[test]
    fn test_package_data_set_description_first_wins() {
        let mut data = PackageData::new();
        data.set_description("First description".to_string());
        data.set_description("Second description".to_string());
        assert_eq!(data.description, Some("First description".to_string()));
    }

    #[test]
    fn test_package_data_set_description_ignores_empty() {
        let mut data = PackageData::new();
        data.set_description("".to_string());
        assert!(data.description.is_none());
    }

    #[test]
    fn test_package_data_is_simple_true() {
        let mut data = PackageData::new();
        data.add_source("brew".to_string(), PackageConfig::Simple);
        data.add_source("scoop".to_string(), PackageConfig::Simple);
        assert!(data.is_simple());
    }

    #[test]
    fn test_package_data_is_simple_false_with_description() {
        let mut data = PackageData::new();
        data.add_source("brew".to_string(), PackageConfig::Simple);
        data.set_description("A description".to_string());
        assert!(!data.is_simple());
    }

    #[test]
    fn test_package_data_is_simple_false_with_name_override() {
        let mut data = PackageData::new();
        data.add_source(
            "brew".to_string(),
            PackageConfig::NameOverride("gh".to_string()),
        );
        assert!(!data.is_simple());
    }

    #[test]
    fn test_package_data_is_simple_false_with_complex_config() {
        let mut data = PackageData::new();
        let mut config = BTreeMap::new();
        config.insert("pre".to_string(), "setup".to_string());
        data.add_source("brew".to_string(), PackageConfig::Complex(config));
        assert!(!data.is_simple());
    }

    #[test]
    fn test_extract_string_value_simple() {
        let document = DocumentMut::parse("test = value").unwrap();
        let item = document.as_table().get("test").unwrap();
        assert_eq!(extract_string_value(item), Some("value".to_string()));
    }

    #[test]
    fn test_extract_string_value_nested_returns_none() {
        let ccl = r#"
test =
  nested = value
"#;
        let document = DocumentMut::parse(ccl).unwrap();
        let item = document.as_table().get("test").unwrap();
        assert_eq!(extract_string_value(item), None);
    }

    #[test]
    fn test_parse_source_content_simple_packages() {
        let ccl = r#"
bat =
fd =
ripgrep =
"#;
        let packages = parse_source_content(ccl).unwrap();
        assert_eq!(packages.len(), 3);
        assert!(matches!(packages["bat"].config, PackageConfig::Simple));
        assert!(matches!(packages["fd"].config, PackageConfig::Simple));
        assert!(matches!(packages["ripgrep"].config, PackageConfig::Simple));
    }

    #[test]
    fn test_parse_source_content_name_override() {
        let ccl = r#"
ripgrep = rg
"#;
        let packages = parse_source_content(ccl).unwrap();
        assert_eq!(packages.len(), 1);
        match &packages["ripgrep"].config {
            PackageConfig::NameOverride(name) => assert_eq!(name, "rg"),
            _ => panic!("Expected NameOverride"),
        }
    }

    #[test]
    fn test_parse_source_content_with_description() {
        let ccl = r#"
bat =
  _description = A cat clone with syntax highlighting
"#;
        let packages = parse_source_content(ccl).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(
            packages["bat"].description,
            Some("A cat clone with syntax highlighting".to_string())
        );
    }

    #[test]
    fn test_parse_source_content_skips_comments() {
        let ccl = r#"
/= This is a comment
bat
/= Another comment
fd
"#;
        let packages = parse_source_content(ccl).unwrap();
        assert_eq!(packages.len(), 2);
        assert!(packages.contains_key("bat"));
        assert!(packages.contains_key("fd"));
        assert!(!packages.contains_key("/= This is a comment"));
    }

    #[test]
    fn test_parse_source_content_complex_config() {
        let ccl = r#"
oh-my-posh =
  pre = setup-fonts
  post = cleanup
"#;
        let packages = parse_source_content(ccl).unwrap();
        assert_eq!(packages.len(), 1);
        match &packages["oh-my-posh"].config {
            PackageConfig::Complex(config) => {
                assert_eq!(config.get("pre"), Some(&"setup-fonts".to_string()));
                assert_eq!(config.get("post"), Some(&"cleanup".to_string()));
            }
            _ => panic!("Expected Complex config"),
        }
    }

    #[test]
    fn test_generate_index_with_temp_dir() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        // Create brew.ccl
        let brew_path = temp_dir.path().join("brew.ccl");
        let mut brew_file = fs::File::create(&brew_path).unwrap();
        writeln!(brew_file, "bat =").unwrap();
        writeln!(brew_file, "fd =").unwrap();

        // Create scoop.ccl
        let scoop_path = temp_dir.path().join("scoop.ccl");
        let mut scoop_file = fs::File::create(&scoop_path).unwrap();
        writeln!(scoop_file, "bat =").unwrap();
        writeln!(scoop_file, "ripgrep = rg").unwrap();

        // Generate the index with empty catalog
        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        // Verify the output contains expected content
        assert!(result.contains("/= Generated package index"));
        assert!(result.contains("bat ="));
        assert!(result.contains("= brew"));
        assert!(result.contains("= scoop"));
    }

    #[test]
    fn test_generate_index_simple_vs_complex_separation() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        // Create a source file with both simple and complex packages
        let source_path = temp_dir.path().join("test.ccl");
        let mut source_file = fs::File::create(&source_path).unwrap();
        writeln!(source_file, "simple-pkg =").unwrap();
        writeln!(source_file, "complex-pkg = override-name").unwrap();

        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        // Simple packages should appear before the complex section header
        assert!(result.contains("/= Packages with simple format"));
        assert!(result.contains("simple-pkg ="));
    }

    #[test]
    fn test_generate_index_preserves_descriptions() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        let source_path = temp_dir.path().join("test.ccl");
        let mut source_file = fs::File::create(&source_path).unwrap();
        writeln!(source_file, "bat =").unwrap();
        writeln!(source_file, "  _description = A cat clone with wings").unwrap();

        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        assert!(result.contains("_description = A cat clone with wings"));
    }

    #[test]
    fn test_generate_index_handles_name_overrides() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        let source_path = temp_dir.path().join("brew.ccl");
        let mut source_file = fs::File::create(&source_path).unwrap();
        writeln!(source_file, "ripgrep = rg").unwrap();

        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        assert!(result.contains("ripgrep ="));
        assert!(result.contains("brew = rg"));
    }

    #[test]
    fn test_generate_index_empty_directory() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        // Should still produce valid output with headers
        assert!(result.contains("/= Generated package index"));
    }

    #[test]
    fn test_generate_index_ignores_non_ccl_files() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        // Create a .ccl file
        let ccl_path = temp_dir.path().join("brew.ccl");
        let mut ccl_file = fs::File::create(&ccl_path).unwrap();
        writeln!(ccl_file, "bat =").unwrap();

        // Create a non-.ccl file
        let txt_path = temp_dir.path().join("readme.txt");
        let mut txt_file = fs::File::create(&txt_path).unwrap();
        writeln!(txt_file, "This should be ignored").unwrap();

        let catalog = BTreeMap::new();
        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        assert!(result.contains("bat ="));
        assert!(!result.contains("readme"));
    }

    #[test]
    fn test_generate_index_uses_catalog_descriptions() {
        use std::io::Write;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();

        // Create a source file with a simple package
        let source_path = temp_dir.path().join("brew.ccl");
        let mut source_file = fs::File::create(&source_path).unwrap();
        writeln!(source_file, "bat =").unwrap();

        // Create a catalog with a description
        let mut catalog = BTreeMap::new();
        catalog.insert(
            "bat".to_string(),
            CatalogEntry {
                description: Some("A cat clone with syntax highlighting".to_string()),
                homepage: None,
                verified: true,
                verified_date: None,
            },
        );

        let (result, _, _) = generate_index(temp_dir.path(), &catalog, true).unwrap();

        // The package should now be complex due to the description
        assert!(result.contains("_description = A cat clone with syntax highlighting"));
    }
}

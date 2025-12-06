//! Generate package_index.ccl from source-organized package files.
//!
//! This binary reads all CCL files in data/sources/ and generates a unified
//! package index that maps package names to their available sources.

use anyhow::{Context, Result};
use sickle::printer::CclPrinter;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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
}

impl PackageData {
    fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
        }
    }

    fn add_source(&mut self, source: String, config: PackageConfig) {
        self.sources.insert(source, config);
    }

    /// Returns true if this is a simple package (all sources have no config)
    fn is_simple(&self) -> bool {
        self.sources
            .values()
            .all(|config| matches!(config, PackageConfig::Simple))
    }
}

/// Extract string from CclObject using the pattern from basic_parsing example
fn extract_string_value(obj: &sickle::CclObject) -> Option<String> {
    if obj.len() == 1 && obj.values().next().unwrap().is_empty() {
        Some(obj.keys().next().unwrap().clone())
    } else {
        None
    }
}

fn parse_source_file(path: &Path) -> Result<BTreeMap<String, PackageConfig>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read source file: {}", path.display()))?;

    let model = sickle::load(&content)
        .with_context(|| format!("Failed to parse CCL in: {}", path.display()))?;

    let mut packages = BTreeMap::new();

    for key in model.keys() {
        // Skip comment lines (keys starting with /)
        if key.starts_with('/') {
            continue;
        }

        let value = model.get(key)?;

        let config = if value.is_empty() {
            // Empty object means simple package (no config)
            PackageConfig::Simple
        } else if let Some(s) = extract_string_value(value) {
            // Empty string means simple package, non-empty means name override
            if s.is_empty() {
                PackageConfig::Simple
            } else {
                PackageConfig::NameOverride(s)
            }
        } else {
            // This is a nested object (complex config)
            let mut config_map = BTreeMap::new();
            for nested_key in value.keys() {
                let nested_value = value.get(nested_key)?;
                if let Some(s) = extract_string_value(nested_value) {
                    config_map.insert(nested_key.clone(), s);
                }
            }
            if config_map.is_empty() {
                PackageConfig::Simple
            } else {
                PackageConfig::Complex(config_map)
            }
        };

        packages.insert(key.clone(), config);
    }

    Ok(packages)
}

fn generate_index(sources_dir: &Path) -> Result<String> {
    let mut all_packages: BTreeMap<String, PackageData> = BTreeMap::new();
    let mut index = sickle::CclObject::new();

    // Read all source files
    for entry in fs::read_dir(sources_dir)
        .with_context(|| format!("Failed to read sources directory: {}", sources_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("ccl") {
            let source_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Invalid source file name")?
                .to_string();

            let packages = parse_source_file(&path)?;

            for (package_name, config) in packages {
                all_packages
                    .entry(package_name)
                    .or_insert_with(PackageData::new)
                    .add_source(source_name.clone(), config);
            }
        }
    }

    // Build CCL structure using sickle's builder API
    let map = index.inner_mut();

    // Add header comments (comment keys with empty values)
    // With Vec-based internal structure, we wrap single values in vec![]
    map.insert("/= Generated package index".to_string(), vec![sickle::CclObject::empty()]);
    map.insert("/= DO NOT EDIT - Generated from data/sources/*.ccl".to_string(), vec![sickle::CclObject::empty()]);
    map.insert("/= Run: just generate-index to regenerate".to_string(), vec![sickle::CclObject::empty()]);
    map.insert("".to_string(), vec![sickle::CclObject::empty()]); // Blank line
    map.insert("/= Packages with simple format (no source-specific overrides)".to_string(), vec![sickle::CclObject::empty()]);

    // Separate simple and complex packages
    let mut simple_packages = Vec::new();
    let mut complex_packages = Vec::new();

    for (name, data) in &all_packages {
        if data.is_simple() {
            simple_packages.push(name);
        } else {
            complex_packages.push(name);
        }
    }

    // Add simple packages
    for package_name in simple_packages {
        let data = &all_packages[package_name];
        let sources: Vec<String> = data.sources.keys().cloned().collect();
        map.insert(package_name.clone(), vec![sickle::CclObject::from_list(sources)]);
    }

    // Add complex packages section header
    if !complex_packages.is_empty() {
        map.insert("".to_string(), vec![sickle::CclObject::empty()]); // Blank line
        map.insert("/= Packages with complex format (have source-specific overrides)".to_string(), vec![sickle::CclObject::empty()]);

        for package_name in complex_packages {
            let data = &all_packages[package_name];
            let mut package_obj = sickle::CclObject::new();
            let package_map = package_obj.inner_mut();

            // Collect sources with no config and sources with config
            let mut simple_sources = Vec::new();
            let mut override_sources = Vec::new();

            for (source, config) in &data.sources {
                match config {
                    PackageConfig::Simple => simple_sources.push(source.clone()),
                    _ => override_sources.push((source, config)),
                }
            }

            // Add source-specific overrides first
            for (source, config) in override_sources {
                match config {
                    PackageConfig::NameOverride(name) => {
                        package_map.insert(source.clone(), vec![sickle::CclObject::from_string(name)]);
                    }
                    PackageConfig::Complex(config_map) => {
                        let mut nested = sickle::CclObject::new();
                        let nested_map = nested.inner_mut();
                        for (key, value) in config_map {
                            nested_map.insert(key.clone(), vec![sickle::CclObject::from_string(value)]);
                        }
                        package_map.insert(source.clone(), vec![nested]);
                    }
                    PackageConfig::Simple => unreachable!(),
                }
            }

            // Add _sources list if there are simple sources
            if !simple_sources.is_empty() {
                package_map.insert("_sources".to_string(), vec![sickle::CclObject::from_list(simple_sources)]);
            }

            map.insert(package_name.clone(), vec![package_obj]);
        }
    }

    // Use CclPrinter to generate the final CCL text
    let printer = CclPrinter::new();
    Ok(printer.print(&index))
}

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sources_dir = manifest_dir.join("data").join("sources");
    let output_file = manifest_dir.join("data").join("known_packages.ccl");

    if !sources_dir.exists() {
        anyhow::bail!("Sources directory not found: {}", sources_dir.display());
    }

    println!("Reading source files from: {}", sources_dir.display());

    // Generate index
    let index_content = generate_index(&sources_dir)?;

    // Write output
    fs::write(&output_file, index_content).context("Failed to write output file")?;

    println!("Generated package index: {}", output_file.display());

    Ok(())
}

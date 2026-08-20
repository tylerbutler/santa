//! Merge verified packages from JSON into per-source CCL files.
//!
//! Reads verified_packages.json and merges new packages into the
//! existing source files in data/sources/. Also updates packages.ccl
//! catalog with descriptions from verified packages.

use anyhow::{Context, Result};
use santa::catalog::{extract_string_value, load_catalog, save_catalog};
use serde::Deserialize;
use sickle::{table, value, DocumentMut, Item};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Verified packages JSON structure
#[derive(Debug, Deserialize)]
struct VerifiedPackages {
    packages: Vec<VerifiedPackage>,
}

#[derive(Debug, Deserialize)]
struct VerifiedPackage {
    name: String,
    description: Option<String>,
    verified_sources: BTreeMap<String, String>,
}

/// Entry for a package in a specific source file
#[derive(Debug, Clone)]
struct SourceEntry {
    name: String,
    override_name: Option<String>,
    config: BTreeMap<String, String>,
}

impl SourceEntry {
    fn new(name: String) -> Self {
        Self {
            name,
            override_name: None,
            config: BTreeMap::new(),
        }
    }

    fn is_simple(&self) -> bool {
        self.override_name.is_none() && self.config.is_empty()
    }
}

/// Result of loading verified packages
struct VerifiedData {
    by_source: BTreeMap<String, Vec<SourceEntry>>,
    descriptions: BTreeMap<String, String>,
    /// Set of package names that have been verified
    verified_names: BTreeSet<String>,
}

/// Load verified packages and group by source, also extract descriptions
fn load_verified_packages(path: &Path) -> Result<VerifiedData> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let data: VerifiedPackages =
        serde_json::from_str(&content).with_context(|| "Failed to parse JSON")?;

    let mut by_source: BTreeMap<String, Vec<SourceEntry>> = BTreeMap::new();
    let mut descriptions: BTreeMap<String, String> = BTreeMap::new();
    let mut verified_names: BTreeSet<String> = BTreeSet::new();

    for pkg in data.packages {
        // Track all verified package names
        verified_names.insert(pkg.name.clone());

        // Collect description if present
        if let Some(ref desc) = pkg.description {
            if !desc.is_empty() {
                descriptions.insert(pkg.name.clone(), desc.clone());
            }
        }

        for (source, source_name) in pkg.verified_sources {
            let mut entry = SourceEntry::new(pkg.name.clone());

            // If source_name differs from package name, it's an override
            if source_name != pkg.name {
                entry.override_name = Some(source_name);
            }

            by_source.entry(source).or_default().push(entry);
        }
    }

    Ok(VerifiedData {
        by_source,
        descriptions,
        verified_names,
    })
}

/// Load existing source file
fn load_existing_source(path: &Path) -> Result<BTreeMap<String, SourceEntry>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let document = DocumentMut::parse(&content)
        .with_context(|| format!("Failed to parse: {}", path.display()))?;

    let mut packages = BTreeMap::new();

    for name in document.as_table().unique_keys() {
        if name.starts_with('/') || name.is_empty() {
            continue;
        }

        let item = document
            .as_table()
            .get(name)
            .expect("key came from the table");
        let mut entry = SourceEntry::new(name.to_string());

        match item {
            Item::Value(scalar) if !scalar.as_str().is_empty() => {
                entry.override_name = Some(scalar.as_str().to_string());
            }
            Item::Table(fields) => {
                for key in fields.unique_keys() {
                    if let Some(val) = fields.get(key).and_then(extract_string_value) {
                        entry.config.insert(key.to_string(), val);
                    }
                }
            }
            _ => {}
        }

        packages.insert(name.to_lowercase(), entry);
    }

    Ok(packages)
}

/// Merge new entries into existing
fn merge_entries(
    existing: BTreeMap<String, SourceEntry>,
    new_entries: Vec<SourceEntry>,
) -> BTreeMap<String, SourceEntry> {
    let mut merged = existing;

    for entry in new_entries {
        let key = entry.name.to_lowercase();
        if let Some(existing_entry) = merged.get_mut(&key) {
            if entry.override_name.is_some() && existing_entry.override_name.is_none() {
                existing_entry.override_name = entry.override_name;
            }
            for (k, v) in entry.config {
                existing_entry.config.entry(k).or_insert(v);
            }
        } else {
            merged.insert(key, entry);
        }
    }

    merged
}

/// Write packages to source CCL file
fn write_source_file(
    path: &Path,
    source_name: &str,
    packages: &BTreeMap<String, SourceEntry>,
) -> Result<()> {
    let mut document = DocumentMut::new();

    // Separate simple and complex entries
    let mut simple: Vec<&SourceEntry> = Vec::new();
    let mut complex: Vec<&SourceEntry> = Vec::new();

    for entry in packages.values() {
        if entry.is_simple() {
            simple.push(entry);
        } else {
            complex.push(entry);
        }
    }

    // Sort by name
    simple.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    complex.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for entry in &simple {
        document
            .as_table_mut()
            .append(entry.name.as_str(), value(""));
    }

    let mut first_complex: Option<String> = None;
    for entry in &complex {
        if let Some(ref override_name) = entry.override_name {
            document
                .as_table_mut()
                .append(entry.name.as_str(), value(override_name));
        } else if !entry.config.is_empty() {
            let mut nested = table();
            let fields = nested.as_table_mut().expect("just created a table");
            for (key, val) in &entry.config {
                fields.append(key.as_str(), value(val));
            }
            document.as_table_mut().append(entry.name.as_str(), nested);
        } else {
            continue;
        }
        first_complex.get_or_insert_with(|| entry.name.clone());
    }

    // Header and section comments are trivia above the entries they introduce.
    if !document.is_empty() {
        document.insert_comment_at(0, &format!("{} packages", capitalize(source_name)))?;
    }
    if let Some(name) = first_complex {
        document.insert_blank_line_before([name.as_str()])?;
        document.insert_comment_before([name.as_str()], "Packages with overrides or config")?;
    }

    fs::write(path, document.to_string())
        .with_context(|| format!("Failed to write: {}", path.display()))?;

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_input = manifest_dir
        .join("data")
        .join("discovery")
        .join("verified_packages.json");

    let input_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        default_input
    };

    let sources_dir = manifest_dir.join("data").join("sources");
    let catalog_file = manifest_dir.join("data").join("packages.ccl");

    if !input_path.exists() {
        anyhow::bail!(
            "verified_packages.json not found: {}\nRun 'just verify-packages' first.",
            input_path.display()
        );
    }

    println!("Loading verified packages from {}...", input_path.display());
    let verified_data = load_verified_packages(&input_path)?;

    if verified_data.by_source.is_empty() {
        println!("No verified sources found in JSON.");
        return Ok(());
    }

    println!(
        "Found packages for sources: {:?}",
        verified_data.by_source.keys().collect::<Vec<_>>()
    );

    // Load existing catalog and merge in new descriptions + verified status
    println!("Loading package catalog from {}...", catalog_file.display());
    let mut catalog = load_catalog(&catalog_file)?;
    let existing_catalog_count = catalog.len();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut new_descriptions = 0;
    let mut newly_verified = 0;

    // Mark all verified packages and add descriptions
    for name in &verified_data.verified_names {
        let entry = catalog.entry(name.clone()).or_default();

        // Add description if we have one and entry doesn't
        if let Some(desc) = verified_data.descriptions.get(name) {
            if entry.description.is_none() {
                entry.description = Some(desc.clone());
                new_descriptions += 1;
            }
        }

        // Mark as verified
        if !entry.verified {
            entry.verified = true;
            entry.verified_date = Some(today.clone());
            newly_verified += 1;
        }
    }

    if new_descriptions > 0 || newly_verified > 0 {
        println!(
            "Catalog updates: {} new descriptions, {} newly verified ({} total entries)",
            new_descriptions,
            newly_verified,
            catalog.len()
        );
        save_catalog(&catalog_file, &catalog)?;
        println!("Updated catalog: {}", catalog_file.display());
    } else {
        println!(
            "No catalog updates needed ({} entries)",
            existing_catalog_count
        );
    }

    fs::create_dir_all(&sources_dir)?;

    let mut total_new = 0;
    for (source, entries) in &verified_data.by_source {
        let source_path = sources_dir.join(format!("{}.ccl", source));

        let existing = load_existing_source(&source_path)?;
        let existing_count = existing.len();

        println!(
            "\n{}: {} existing, {} verified",
            source,
            existing_count,
            entries.len()
        );

        let merged = merge_entries(existing, entries.clone());
        let new_count = merged.len() - existing_count;
        total_new += new_count;

        println!("{}: {} total (+{} new)", source, merged.len(), new_count);

        write_source_file(&source_path, source, &merged)?;
        println!("Written to {}", source_path.display());
    }

    println!(
        "\nAdded {} new packages across {} sources",
        total_new,
        verified_data.by_source.len()
    );

    Ok(())
}

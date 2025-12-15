//! Migrate packages from known_packages.ccl to per-source CCL files.
//!
//! This is the reverse operation of generate_index - it reads the unified
//! package index and distributes packages to their respective source files.

use anyhow::{Context, Result};
use sickle::printer::CclPrinter;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Entry for a package in a specific source file
#[derive(Debug, Clone)]
struct SourceEntry {
    /// Package name
    name: String,
    /// Override name in this source (e.g., ripgrep -> rg)
    override_name: Option<String>,
    /// Complex config (pre, post, install_suffix, etc.)
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

/// Extract string from CclObject (single key with empty value)
fn extract_string_value(obj: &sickle::CclObject) -> Option<String> {
    if obj.len() == 1 && obj.values().next().unwrap().is_empty() {
        Some(obj.keys().next().unwrap().clone())
    } else {
        None
    }
}

/// Parse known_packages.ccl and return packages grouped by source
fn parse_known_packages(path: &Path) -> Result<BTreeMap<String, Vec<SourceEntry>>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let model = sickle::load(&content)
        .with_context(|| format!("Failed to parse CCL: {}", path.display()))?;

    let mut by_source: BTreeMap<String, Vec<SourceEntry>> = BTreeMap::new();

    for package_name in model.keys() {
        // Skip comments
        if package_name.starts_with('/') || package_name.is_empty() {
            continue;
        }

        let value = model.get(package_name)?;

        if value.is_empty() {
            continue;
        }

        let mut found_sources = false;

        // Check for list format (empty key with Vec of source CclObjects)
        // This handles: package = \n  = brew \n  = scoop
        if let Ok(list_values) = value.get_all("") {
            for source_obj in list_values {
                // Each source_obj has the source name as its key
                for source in source_obj.keys() {
                    if !source.is_empty() && !source.starts_with('/') {
                        by_source
                            .entry(source.clone())
                            .or_default()
                            .push(SourceEntry::new(package_name.clone()));
                        found_sources = true;
                    }
                }
            }
        }

        // Check for complex format with named sources
        for key in value.keys() {
            if key.is_empty() || key.starts_with('/') {
                continue;
            }

            let nested = value.get(key)?;

            if key == "_sources" {
                // List of simple sources in complex package
                if let Ok(source_list) = nested.get_all("") {
                    for source_obj in source_list {
                        for source in source_obj.keys() {
                            if !source.is_empty() {
                                by_source
                                    .entry(source.clone())
                                    .or_default()
                                    .push(SourceEntry::new(package_name.clone()));
                                found_sources = true;
                            }
                        }
                    }
                }
            } else if key == "_description" || key.starts_with('_') {
                continue;
            } else if let Some(override_name) = extract_string_value(nested) {
                // Source with name override: brew = rg
                let mut entry = SourceEntry::new(package_name.clone());
                entry.override_name = Some(override_name);
                by_source.entry(key.clone()).or_default().push(entry);
                found_sources = true;
            } else if !nested.is_empty() {
                // Complex config: brew = \n  pre = something
                let mut entry = SourceEntry::new(package_name.clone());
                for config_key in nested.keys() {
                    if !config_key.is_empty() {
                        if let Some(config_val) = extract_string_value(nested.get(config_key)?) {
                            entry.config.insert(config_key.clone(), config_val);
                        }
                    }
                }
                if !entry.config.is_empty() {
                    by_source.entry(key.clone()).or_default().push(entry);
                    found_sources = true;
                }
            }
        }

        if !found_sources {
            eprintln!("Warning: no sources found for package: {}", package_name);
        }
    }

    Ok(by_source)
}

/// Load existing source file
fn load_existing_source(path: &Path) -> Result<BTreeMap<String, SourceEntry>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let model =
        sickle::load(&content).with_context(|| format!("Failed to parse: {}", path.display()))?;

    let mut packages = BTreeMap::new();

    for name in model.keys() {
        if name.starts_with('/') || name.is_empty() {
            continue;
        }

        let value = model.get(name)?;
        let mut entry = SourceEntry::new(name.clone());

        if let Some(override_name) = extract_string_value(value) {
            if !override_name.is_empty() {
                entry.override_name = Some(override_name);
            }
        } else if !value.is_empty() {
            // Complex config
            for key in value.keys() {
                if let Some(val) = extract_string_value(value.get(key)?) {
                    entry.config.insert(key.clone(), val);
                }
            }
        }

        packages.insert(name.to_lowercase(), entry);
    }

    Ok(packages)
}

/// Merge new entries into existing, preserving existing config
fn merge_entries(
    existing: BTreeMap<String, SourceEntry>,
    new_entries: Vec<SourceEntry>,
) -> BTreeMap<String, SourceEntry> {
    let mut merged = existing;

    for entry in new_entries {
        let key = entry.name.to_lowercase();
        if let Some(existing_entry) = merged.get_mut(&key) {
            // Update if new has more info
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
    let mut obj = sickle::CclObject::new();
    obj.add_comment(&format!("{} packages", capitalize(source_name)));

    let map = obj.inner_mut();

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

    // Add simple packages
    for entry in &simple {
        map.insert(entry.name.clone(), vec![sickle::CclObject::empty()]);
    }

    // Add complex packages
    if !complex.is_empty() {
        map.insert(String::new(), vec![sickle::CclObject::empty()]);
        map.insert(
            "/= Packages with overrides or config".to_string(),
            vec![sickle::CclObject::empty()],
        );

        for entry in &complex {
            if let Some(ref override_name) = entry.override_name {
                map.insert(
                    entry.name.clone(),
                    vec![sickle::CclObject::from_string(override_name)],
                );
            } else if !entry.config.is_empty() {
                let mut nested = sickle::CclObject::new();
                let nested_map = nested.inner_mut();
                for (k, v) in &entry.config {
                    nested_map.insert(k.clone(), vec![sickle::CclObject::from_string(v)]);
                }
                map.insert(entry.name.clone(), vec![nested]);
            }
        }
    }

    let printer = CclPrinter::new();
    let output = printer.print(&obj);
    fs::write(path, output).with_context(|| format!("Failed to write: {}", path.display()))?;

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
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let known_packages = manifest_dir.join("data").join("known_packages.ccl");
    let sources_dir = manifest_dir.join("data").join("sources");

    if !known_packages.exists() {
        anyhow::bail!("known_packages.ccl not found: {}", known_packages.display());
    }

    println!("Parsing {}...", known_packages.display());
    let by_source = parse_known_packages(&known_packages)?;

    println!("Found {} sources:", by_source.len());
    for (source, entries) in &by_source {
        println!("  {}: {} packages", source, entries.len());
    }

    fs::create_dir_all(&sources_dir)?;

    let mut total = 0;
    for (source, entries) in &by_source {
        let source_path = sources_dir.join(format!("{}.ccl", source));

        // Load existing
        let existing = load_existing_source(&source_path)?;
        let existing_count = existing.len();

        // Merge
        let merged = merge_entries(existing, entries.clone());
        let new_count = merged.len() - existing_count;

        println!(
            "\n{}: {} existing, {} from index",
            source,
            existing_count,
            entries.len()
        );
        println!("{}: {} total (+{} new)", source, merged.len(), new_count);

        write_source_file(&source_path, source, &merged)?;
        println!("Written to {}", source_path.display());

        total += merged.len();
    }

    println!(
        "\nTotal: {} package entries across {} sources",
        total,
        by_source.len()
    );

    Ok(())
}

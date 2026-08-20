//! Migrate packages from known_packages.ccl to per-source CCL files.
//!
//! This is the reverse operation of generate_index - it reads the unified
//! package index and distributes packages to their respective source files.

use anyhow::{Context, Result};
use sickle::{table, value, DocumentMut, Item};
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

/// Extract a scalar value from a CCL node.
fn extract_string_value(item: &Item) -> Option<String> {
    item.as_str().map(str::to_string)
}

/// Parse known_packages.ccl and return packages grouped by source
fn parse_known_packages(path: &Path) -> Result<BTreeMap<String, Vec<SourceEntry>>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let document = DocumentMut::parse(&content)
        .with_context(|| format!("Failed to parse CCL: {}", path.display()))?;

    let mut by_source: BTreeMap<String, Vec<SourceEntry>> = BTreeMap::new();

    for package_name in document.as_table().unique_keys() {
        // Skip comment-shaped keys and bare list items
        if package_name.starts_with('/') || package_name.is_empty() {
            continue;
        }

        let Some(package) = document.as_table().get(package_name) else {
            continue;
        };

        let mut found_sources = false;

        if let Some(fields) = package.as_table() {
            for key in fields.unique_keys() {
                if key.starts_with('/') {
                    continue;
                }

                let item = fields.get(key).expect("key came from the table");

                if key.is_empty() {
                    continue;
                }

                if key == "_sources" {
                    // List of simple sources inside a complex package
                    if let Some(sources) = item.as_array() {
                        for source in sources.iter().filter_map(|i| i.as_str()) {
                            by_source
                                .entry(source.to_string())
                                .or_default()
                                .push(SourceEntry::new(package_name.to_string()));
                            found_sources = true;
                        }
                    }
                } else if key.starts_with('_') {
                    continue;
                } else if let Some(override_name) = extract_string_value(item) {
                    // Source with a name override: `brew = rg`
                    let mut entry = SourceEntry::new(package_name.to_string());
                    if !override_name.is_empty() {
                        entry.override_name = Some(override_name);
                    }
                    by_source.entry(key.to_string()).or_default().push(entry);
                    found_sources = true;
                } else if let Some(config) = item.as_table() {
                    // Complex config: `brew =` followed by an indented block
                    let mut entry = SourceEntry::new(package_name.to_string());
                    for config_key in config.unique_keys() {
                        if config_key.is_empty() {
                            continue;
                        }
                        if let Some(val) = config.get(config_key).and_then(extract_string_value) {
                            entry.config.insert(config_key.to_string(), val);
                        }
                    }
                    if !entry.config.is_empty() {
                        by_source.entry(key.to_string()).or_default().push(entry);
                        found_sources = true;
                    }
                }
            }
        }

        // Simple format: `package =` followed by an indented bare list.
        if let Some(sources) = package.as_array() {
            for source in sources.iter().filter_map(|i| i.as_str()) {
                if !source.is_empty() && !source.starts_with('/') {
                    by_source
                        .entry(source.to_string())
                        .or_default()
                        .push(SourceEntry::new(package_name.to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_bare_list_packages() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("known_packages.ccl");
        let by_source = parse_known_packages(&path).unwrap();

        for source in ["brew", "scoop"] {
            assert!(
                by_source[source].iter().any(|entry| entry.name == "act"),
                "expected simple package act in {source}"
            );
        }
    }
}

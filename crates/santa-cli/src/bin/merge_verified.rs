//! Merge verified packages from JSON into per-source CCL files.
//!
//! Reads verified_packages.json and merges new packages into the
//! existing source files in data/sources/.

use anyhow::{Context, Result};
use serde::Deserialize;
use sickle::printer::CclPrinter;
use std::collections::BTreeMap;
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
    #[allow(dead_code)]
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

/// Extract string from CclObject
fn extract_string_value(obj: &sickle::CclObject) -> Option<String> {
    if obj.len() == 1 && obj.values().next().unwrap().is_empty() {
        Some(obj.keys().next().unwrap().clone())
    } else {
        None
    }
}

/// Load verified packages and group by source
fn load_verified_packages(path: &Path) -> Result<BTreeMap<String, Vec<SourceEntry>>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read: {}", path.display()))?;

    let data: VerifiedPackages =
        serde_json::from_str(&content).with_context(|| "Failed to parse JSON")?;

    let mut by_source: BTreeMap<String, Vec<SourceEntry>> = BTreeMap::new();

    for pkg in data.packages {
        for (source, source_name) in pkg.verified_sources {
            let mut entry = SourceEntry::new(pkg.name.clone());

            // If source_name differs from package name, it's an override
            if source_name != pkg.name {
                entry.override_name = Some(source_name);
            }

            by_source.entry(source).or_default().push(entry);
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
    let mut obj = sickle::CclObject::new();
    obj.add_comment(&format!("{} packages", capitalize(source_name)));

    let map = obj.inner_mut();

    let mut simple: Vec<&SourceEntry> = Vec::new();
    let mut complex: Vec<&SourceEntry> = Vec::new();

    for entry in packages.values() {
        if entry.is_simple() {
            simple.push(entry);
        } else {
            complex.push(entry);
        }
    }

    simple.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    complex.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for entry in &simple {
        map.insert(entry.name.clone(), vec![sickle::CclObject::empty()]);
    }

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

    if !input_path.exists() {
        anyhow::bail!(
            "verified_packages.json not found: {}\nRun 'just verify-packages' first.",
            input_path.display()
        );
    }

    println!("Loading verified packages from {}...", input_path.display());
    let by_source = load_verified_packages(&input_path)?;

    if by_source.is_empty() {
        println!("No verified sources found in JSON.");
        return Ok(());
    }

    println!(
        "Found packages for sources: {:?}",
        by_source.keys().collect::<Vec<_>>()
    );

    fs::create_dir_all(&sources_dir)?;

    let mut total_new = 0;
    for (source, entries) in &by_source {
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
        by_source.len()
    );

    Ok(())
}

//! Fetch package name mappings from Repology.
//!
//! Queries the Repology API to discover how a package is named across
//! different package managers, and optionally updates source CCL files.
//!
//! Usage:
//!   fetch-repology <project-name>           # Query and display mappings
//!   fetch-repology <project-name> --update  # Update source CCL files
//!   fetch-repology --batch packages.txt     # Process multiple packages

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use sickle::CclObject;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Rate limit: 1 request per second per Repology guidelines
const RATE_LIMIT_MS: u64 = 1100;

/// User agent as required by Repology API
const USER_AGENT: &str = "santa-package-manager/0.1 (https://github.com/tylerbutler/santa)";

/// Repology repository identifiers mapped to our source names
const REPO_MAPPING: &[(&str, &str)] = &[
    // Homebrew
    ("homebrew", "brew"),
    ("homebrew_casks", "brew"),
    // Linux
    ("debian_12", "apt"),
    ("debian_13", "apt"),
    ("ubuntu_24_04", "apt"),
    ("arch", "pacman"),
    ("aur", "aur"),
    ("nix_unstable", "nix"),
    ("nix_stable_24_05", "nix"),
    // Windows
    ("scoop", "scoop"),
    ("chocolatey", "choco"),
    // Language package managers
    ("crates_io", "cargo"),
    ("npm", "npm"),
    ("pypi", "pip"),
];

/// Package entry from Repology API
#[derive(Debug, Deserialize)]
struct RepologyPackage {
    repo: String,
    #[serde(default)]
    srcname: Option<String>,
    #[serde(default)]
    binname: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    summary: Option<String>,
}

/// Resolved package info for a source
#[derive(Debug, Clone)]
struct SourceMapping {
    #[allow(dead_code)]
    source: String,
    package_name: String,
    version: Option<String>,
    is_newest: bool,
}

/// Query Repology for a project and return package mappings
async fn fetch_repology_project(
    client: &reqwest::Client,
    project: &str,
) -> Result<Vec<RepologyPackage>> {
    let url = format!("https://repology.org/api/v1/project/{}", project);

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("HTTP request failed for: {}", url))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !response.status().is_success() {
        anyhow::bail!(
            "Repology API returned status {}: {}",
            response.status(),
            url
        );
    }

    let packages: Vec<RepologyPackage> = response
        .json()
        .await
        .with_context(|| format!("Failed to parse Repology response for {}", project))?;

    Ok(packages)
}

/// Extract the best package name from a Repology entry
fn get_package_name(pkg: &RepologyPackage) -> Option<String> {
    // Prefer binname, then srcname
    pkg.binname
        .clone()
        .or_else(|| pkg.srcname.clone())
        .map(|name| {
            // Strip category prefixes like "dev-util/", "net/", etc.
            if let Some(idx) = name.rfind('/') {
                name[idx + 1..].to_string()
            } else {
                name
            }
        })
}

/// Map Repology packages to our source format
fn map_to_sources(packages: Vec<RepologyPackage>) -> BTreeMap<String, SourceMapping> {
    let mut mappings: BTreeMap<String, SourceMapping> = BTreeMap::new();

    // Build a lookup from repology repo -> our source name
    let repo_map: BTreeMap<&str, &str> = REPO_MAPPING.iter().cloned().collect();

    for pkg in packages {
        // Find if this repo maps to one of our sources
        let source = repo_map
            .iter()
            .find(|(repology_repo, _)| pkg.repo.starts_with(*repology_repo))
            .map(|(_, our_source)| *our_source);

        if let Some(source) = source {
            if let Some(name) = get_package_name(&pkg) {
                let is_newest = pkg.status.as_deref() == Some("newest");

                // Keep the "newest" version if we have multiple entries for same source
                let should_update = match mappings.get(source) {
                    None => true,
                    Some(existing) => !existing.is_newest && is_newest,
                };

                if should_update {
                    mappings.insert(
                        source.to_string(),
                        SourceMapping {
                            source: source.to_string(),
                            package_name: name,
                            version: pkg.version,
                            is_newest,
                        },
                    );
                }
            }
        }
    }

    mappings
}

/// Format package mappings for display
fn display_mappings(project: &str, mappings: &BTreeMap<String, SourceMapping>) {
    if mappings.is_empty() {
        println!("No packages found for project: {}", project);
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("Project: {}", project);
    println!("{}", "=".repeat(60));

    // Group by whether name differs from project name
    let mut same_name = Vec::new();
    let mut different_name = Vec::new();

    for (source, mapping) in mappings {
        if mapping.package_name == project {
            same_name.push(source.as_str());
        } else {
            different_name.push((source.as_str(), &mapping.package_name, &mapping.version));
        }
    }

    if !same_name.is_empty() {
        println!("\nAvailable as '{}' in:", project);
        for source in &same_name {
            println!("  - {}", source);
        }
    }

    if !different_name.is_empty() {
        println!("\nDifferent names:");
        for (source, name, version) in &different_name {
            let ver = version
                .as_ref()
                .map(|v| format!(" ({})", v))
                .unwrap_or_default();
            println!("  {} → {}{}", source, name, ver);
        }
    }
}

/// Generate CCL entries for updating source files
#[allow(dead_code)]
fn generate_ccl_entries(
    project: &str,
    mappings: &BTreeMap<String, SourceMapping>,
) -> BTreeMap<String, String> {
    let mut entries: BTreeMap<String, String> = BTreeMap::new();

    for (source, mapping) in mappings {
        let entry = if mapping.package_name == project {
            // Same name - simple entry
            format!("{} =", mapping.package_name)
        } else {
            // Different name - needs mapping
            format!("{} = {}", mapping.package_name, project)
        };
        entries.insert(source.clone(), entry);
    }

    entries
}

/// Update source CCL files with new mappings
fn update_source_files(
    sources_dir: &Path,
    project: &str,
    mappings: &BTreeMap<String, SourceMapping>,
) -> Result<Vec<String>> {
    let mut updated = Vec::new();

    for (source, mapping) in mappings {
        let source_file = sources_dir.join(format!("{}.ccl", source));

        if !source_file.exists() {
            println!("  [skip] {}.ccl not found", source);
            continue;
        }

        let content = fs::read_to_string(&source_file)?;

        // Check if package already exists in file
        let package_name = &mapping.package_name;
        let already_exists = content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with(&format!("{} =", package_name))
                || trimmed.starts_with(&format!("{}=", package_name))
        });

        if already_exists {
            println!("  [exists] {} in {}.ccl", package_name, source);
            continue;
        }

        // Generate the new entry
        let new_entry = if mapping.package_name == project {
            format!("{} =\n", package_name)
        } else {
            format!("{} = {}\n", package_name, project)
        };

        // Append to file (we'll rely on generate-index to sort later)
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&new_entry);

        fs::write(&source_file, new_content)?;
        println!("  [added] {} to {}.ccl", package_name, source);
        updated.push(format!("{}.ccl", source));
    }

    Ok(updated)
}

/// Read package list from file (one per line)
fn read_package_list(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect())
}

/// Crossref results JSON structure
#[derive(Debug, Deserialize)]
struct CrossrefResults {
    packages: Vec<CrossrefPackage>,
}

#[derive(Debug, Deserialize)]
struct CrossrefPackage {
    name: String,
}

/// Extract string value from CCL object (pattern from generate_index.rs)
fn extract_string_value(obj: &CclObject) -> Option<String> {
    if obj.len() == 1 && obj.values().next().unwrap().is_empty() {
        Some(obj.keys().next().unwrap().clone())
    } else {
        None
    }
}

/// Read verified packages from the catalog using sickle
fn get_verified_packages(catalog_path: &Path) -> Result<BTreeSet<String>> {
    let mut verified = BTreeSet::new();

    if !catalog_path.exists() {
        return Ok(verified);
    }

    let content = fs::read_to_string(catalog_path)?;
    let model = sickle::load(&content)
        .with_context(|| format!("Failed to parse catalog: {}", catalog_path.display()))?;

    for key in model.keys() {
        // Skip comments and empty keys
        if key.starts_with('/') || key.is_empty() {
            continue;
        }

        if let Ok(value) = model.get(key) {
            // Check if this package has a verified field
            if value.get("verified").is_ok() {
                verified.insert(key.clone());
            }
        }
    }

    Ok(verified)
}

/// Get all source file names from the sources directory
fn get_all_source_names(sources_dir: &Path) -> Result<Vec<String>> {
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

/// Read package names from crossref_results.json
fn read_crossref_packages(path: &Path, limit: usize) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read crossref file: {}", path.display()))?;

    let results: CrossrefResults = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse crossref JSON: {}", path.display()))?;

    Ok(results
        .packages
        .into_iter()
        .take(limit)
        .map(|p| p.name)
        .collect())
}

/// Package entry from a source CCL file
#[derive(Debug, Clone)]
struct SourceEntry {
    /// The package name as it appears in the source (e.g., "gh" in brew)
    source_name: String,
    /// The canonical name it maps to (e.g., "github-cli"), or same as source_name
    canonical_name: String,
    /// Which source file this came from
    source: String,
}

/// Read all packages from a source CCL file using sickle
fn read_source_ccl(path: &Path, source_name: &str) -> Result<Vec<SourceEntry>> {
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

        // Determine canonical name:
        // - If value is empty object, canonical = source_name (e.g., "bat =")
        // - If value is a string, that's the canonical (e.g., "gh = github-cli")
        // - If value is a nested object, check for nested keys
        let canonical = if value.is_empty() {
            // Empty value means same name
            key.clone()
        } else if let Some(s) = extract_string_value(value) {
            if s.is_empty() {
                key.clone()
            } else {
                s
            }
        } else {
            // Nested object - canonical is the key itself
            key.clone()
        };

        entries.push(SourceEntry {
            source_name: key.clone(),
            canonical_name: canonical,
            source: source_name.to_string(),
        });
    }

    Ok(entries)
}

/// Validation result for a single package
#[derive(Debug)]
enum ValidationResult {
    Ok,
    NotFound,
    Mismatch {
        expected: String,
        actual: String,
    },
    Missing {
        repology_name: String,
    },
}

/// Validated package info to write to catalog
#[derive(Debug)]
struct ValidatedPackage {
    #[allow(dead_code)]
    canonical_name: String,
    repology_project: Option<String>, // Only if different from canonical
    verified_date: String,
}

/// Validate source entries against Repology
async fn validate_sources(
    client: &reqwest::Client,
    sources_dir: &Path,
    catalog_path: &Path,
    source_names: &[String],
    force: bool,
) -> Result<()> {
    // Collect all entries from all source files
    let mut all_entries: Vec<SourceEntry> = Vec::new();

    for source_name in source_names {
        let source_path = sources_dir.join(format!("{}.ccl", source_name));
        if !source_path.exists() {
            println!("[WARN] Source file not found: {}.ccl", source_name);
            continue;
        }

        let entries = read_source_ccl(&source_path, source_name)?;
        println!("Loaded {} entries from {}.ccl", entries.len(), source_name);
        all_entries.extend(entries);
    }

    if all_entries.is_empty() {
        println!("No entries to validate");
        return Ok(());
    }

    // Get unique canonical names to minimize API calls
    let mut unique_canonicals: BTreeSet<String> = all_entries
        .iter()
        .map(|e| e.canonical_name.clone())
        .collect();

    // Filter out already-verified packages unless force is set
    let mut skipped_count = 0;
    if !force {
        let already_verified = get_verified_packages(catalog_path)?;
        let before_count = unique_canonicals.len();
        unique_canonicals.retain(|name| !already_verified.contains(name));
        skipped_count = before_count - unique_canonicals.len();

        // Also filter entries to only those we're validating
        all_entries.retain(|e| unique_canonicals.contains(&e.canonical_name));

        if skipped_count > 0 {
            println!(
                "Skipping {} already-verified packages (use --force to re-validate)",
                skipped_count
            );
        }
    }

    if unique_canonicals.is_empty() {
        println!("\nNo packages to validate (all {} are already verified)", skipped_count);
        return Ok(());
    }

    println!(
        "\nValidating {} unique packages across {} source entries...\n",
        unique_canonicals.len(),
        all_entries.len()
    );

    // Fetch Repology data for each unique canonical name
    let mut repology_cache: BTreeMap<String, BTreeMap<String, SourceMapping>> = BTreeMap::new();
    let mut not_found_in_repology: BTreeSet<String> = BTreeSet::new();

    for (i, canonical) in unique_canonicals.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
        }

        print!(
            "[{}/{}] Fetching {} ... ",
            i + 1,
            unique_canonicals.len(),
            canonical
        );

        match fetch_repology_project(client, canonical).await {
            Ok(packages) => {
                if packages.is_empty() {
                    println!("not found");
                    not_found_in_repology.insert(canonical.clone());
                } else {
                    let mappings = map_to_sources(packages);
                    println!("found in {} sources", mappings.len());
                    repology_cache.insert(canonical.clone(), mappings);
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    // Now validate each entry
    println!("\n{}", "=".repeat(60));
    println!("Validation Results");
    println!("{}", "=".repeat(60));

    let mut stats = ValidationStats::default();

    // Group entries by source for organized output
    let mut by_source: BTreeMap<String, Vec<&SourceEntry>> = BTreeMap::new();
    for entry in &all_entries {
        by_source
            .entry(entry.source.clone())
            .or_default()
            .push(entry);
    }

    for (source, entries) in &by_source {
        println!("\n{}:", source);

        for entry in entries {
            let result = validate_entry(entry, &repology_cache, &not_found_in_repology);

            match &result {
                ValidationResult::Ok => {
                    stats.ok += 1;
                    // Don't print OK entries to reduce noise
                }
                ValidationResult::NotFound => {
                    stats.not_found += 1;
                    println!(
                        "  [NOT FOUND] {} (canonical: {})",
                        entry.source_name, entry.canonical_name
                    );
                }
                ValidationResult::Mismatch { expected, actual } => {
                    stats.mismatch += 1;
                    println!(
                        "  [MISMATCH] {} - expected '{}', Repology says '{}'",
                        entry.canonical_name, expected, actual
                    );
                }
                ValidationResult::Missing { repology_name } => {
                    stats.missing += 1;
                    println!(
                        "  [MISSING] {} should be '{}' (per Repology)",
                        entry.canonical_name, repology_name
                    );
                }
            }
        }

        // Count OK for this source
        let ok_count = entries
            .iter()
            .filter(|e| {
                matches!(
                    validate_entry(e, &repology_cache, &not_found_in_repology),
                    ValidationResult::Ok
                )
            })
            .count();
        if ok_count > 0 {
            println!("  [{} OK]", ok_count);
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("Summary:");
    println!("  OK: {}", stats.ok);
    println!("  Not found in Repology: {}", stats.not_found);
    println!("  Name mismatches: {}", stats.mismatch);
    println!("  Missing mappings: {}", stats.missing);

    // Collect validated packages for catalog update
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut validated: BTreeMap<String, ValidatedPackage> = BTreeMap::new();

    for canonical in repology_cache.keys() {
        // Only mark as verified if it was found in Repology
        validated.insert(
            canonical.clone(),
            ValidatedPackage {
                canonical_name: canonical.clone(),
                repology_project: None, // Same as canonical for now
                verified_date: today.clone(),
            },
        );
    }

    // Update catalog with verified status
    if !validated.is_empty() {
        println!("\nUpdating catalog with {} verified packages...", validated.len());
        update_catalog_verified(catalog_path, &validated)?;
        println!("Catalog updated: {}", catalog_path.display());
    }

    Ok(())
}

#[derive(Default)]
struct ValidationStats {
    ok: usize,
    not_found: usize,
    mismatch: usize,
    missing: usize,
}

/// Validate a single entry against Repology data
fn validate_entry(
    entry: &SourceEntry,
    repology_cache: &BTreeMap<String, BTreeMap<String, SourceMapping>>,
    not_found: &BTreeSet<String>,
) -> ValidationResult {
    // If canonical name wasn't found in Repology
    if not_found.contains(&entry.canonical_name) {
        return ValidationResult::NotFound;
    }

    // Get Repology's mappings for this canonical name
    let Some(repology_mappings) = repology_cache.get(&entry.canonical_name) else {
        return ValidationResult::NotFound;
    };

    // Check if Repology has this source
    let Some(repology_entry) = repology_mappings.get(&entry.source) else {
        // Repology doesn't have this package for this source - that's OK,
        // we might have it from a different data source
        return ValidationResult::Ok;
    };

    // Compare our source name with Repology's
    if entry.source_name == repology_entry.package_name {
        ValidationResult::Ok
    } else if entry.source_name == entry.canonical_name {
        // We have "pkg =" but Repology says it should be different
        ValidationResult::Missing {
            repology_name: repology_entry.package_name.clone(),
        }
    } else {
        // We have a mapping but it doesn't match Repology
        ValidationResult::Mismatch {
            expected: entry.source_name.clone(),
            actual: repology_entry.package_name.clone(),
        }
    }
}

/// Update the catalog file with verified status for packages
fn update_catalog_verified(
    catalog_path: &Path,
    validated: &BTreeMap<String, ValidatedPackage>,
) -> Result<()> {
    let content = fs::read_to_string(catalog_path)
        .with_context(|| format!("Failed to read catalog: {}", catalog_path.display()))?;

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut updated_packages: BTreeSet<String> = BTreeSet::new();

    // Process the file to update existing entries
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('/') {
            i += 1;
            continue;
        }

        // Check if this is a package entry (not indented, has =)
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(eq_pos) = trimmed.find('=') {
                let pkg_name = trimmed[..eq_pos].trim();

                if let Some(validated_pkg) = validated.get(pkg_name) {
                    updated_packages.insert(pkg_name.to_string());

                    // Find the extent of this package's block
                    let mut block_end = i + 1;
                    while block_end < lines.len() {
                        let next_line = &lines[block_end];
                        // Block ends at next non-indented line or end of file
                        if !next_line.is_empty()
                            && !next_line.starts_with(' ')
                            && !next_line.starts_with('\t')
                        {
                            break;
                        }
                        block_end += 1;
                    }

                    // Check if verified field already exists in this block
                    let mut has_verified = false;
                    let mut verified_line_idx = None;
                    for j in (i + 1)..block_end {
                        if lines[j].trim().starts_with("verified =") {
                            has_verified = true;
                            verified_line_idx = Some(j);
                            break;
                        }
                    }

                    if has_verified {
                        // Update existing verified line
                        if let Some(idx) = verified_line_idx {
                            lines[idx] = format!("  verified = {}", validated_pkg.verified_date);
                        }
                    } else {
                        // Add verified field - insert before block_end
                        let insert_idx = if block_end > i + 1 { block_end } else { i + 1 };

                        // Build new fields to insert
                        let mut new_fields = Vec::new();
                        if let Some(ref repology) = validated_pkg.repology_project {
                            new_fields.push(format!("  repology = {}", repology));
                        }
                        new_fields.push(format!("  verified = {}", validated_pkg.verified_date));

                        // Insert in reverse order to maintain indices
                        for field in new_fields.into_iter().rev() {
                            lines.insert(insert_idx, field);
                        }
                    }

                    // Skip to end of this block
                    i = block_end;
                    continue;
                }
            }
        }

        i += 1;
    }

    // Add new packages that weren't in the catalog
    let mut new_entries = Vec::new();
    for (name, pkg) in validated {
        if !updated_packages.contains(name) {
            let mut entry = format!("\n{} =", name);
            if let Some(ref repology) = pkg.repology_project {
                entry.push_str(&format!("\n  repology = {}", repology));
            }
            entry.push_str(&format!("\n  verified = {}", pkg.verified_date));
            new_entries.push(entry);
        }
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    for entry in new_entries {
        output.push_str(&entry);
    }
    if !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(catalog_path, output)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!(
            "  {} <project-name>             Query and display mappings",
            args[0]
        );
        eprintln!(
            "  {} <project-name> --update    Update source CCL files",
            args[0]
        );
        eprintln!(
            "  {} --batch <file>             Process multiple packages",
            args[0]
        );
        eprintln!(
            "  {} --from-crossref [limit]    Process top packages from crossref_results.json",
            args[0]
        );
        eprintln!(
            "  {} --validate <sources...>    Validate source files against Repology",
            args[0]
        );
        eprintln!("                                  (e.g., --validate all, --validate brew apt)");
        eprintln!(
            "  {} --validate all --force     Re-validate all packages (ignore verified status)",
            args[0]
        );
        std::process::exit(1);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sources_dir = manifest_dir.join("data").join("sources");
    let crossref_path = manifest_dir
        .join("data")
        .join("discovery")
        .join("crossref_results.json");

    let is_batch = args.iter().any(|a| a == "--batch");
    let is_crossref = args.iter().any(|a| a == "--from-crossref");
    let is_validate = args.iter().any(|a| a == "--validate");
    let do_update = args.iter().any(|a| a == "--update");
    let force = args.iter().any(|a| a == "--force");

    // Handle validation mode
    if is_validate {
        let validate_pos = args.iter().position(|a| a == "--validate").unwrap();
        let mut source_names: Vec<String> = args[validate_pos + 1..]
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned()
            .collect();

        // If "all" is specified, get all source files
        if source_names.iter().any(|s| s == "all") {
            source_names = get_all_source_names(&sources_dir)?;
        }

        if source_names.is_empty() {
            eprintln!("--validate requires at least one source name (e.g., brew, apt, nix) or 'all'");
            std::process::exit(1);
        }

        let client = reqwest::Client::new();
        let catalog_path = manifest_dir.join("data").join("packages.ccl");
        return validate_sources(&client, &sources_dir, &catalog_path, &source_names, force).await;
    }

    // Collect projects to process
    let projects: Vec<String> = if is_crossref {
        // Get optional limit argument after --from-crossref
        let limit = args
            .iter()
            .position(|a| a == "--from-crossref")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100);
        read_crossref_packages(&crossref_path, limit)?
    } else if is_batch {
        let file_arg = args
            .iter()
            .position(|a| a == "--batch")
            .and_then(|i| args.get(i + 1))
            .ok_or_else(|| anyhow::anyhow!("--batch requires a file path"))?;
        read_package_list(&PathBuf::from(file_arg))?
    } else {
        vec![args[1].clone()]
    };

    if projects.is_empty() {
        println!("No packages to process");
        return Ok(());
    }

    println!("Querying Repology for {} package(s)...", projects.len());
    if do_update {
        println!("Will update source files in: {}", sources_dir.display());
    }

    let client = reqwest::Client::new();
    let mut total_updated = 0;
    let mut not_found: Vec<String> = Vec::new();

    for (i, project) in projects.iter().enumerate() {
        if i > 0 {
            // Rate limiting
            tokio::time::sleep(Duration::from_millis(RATE_LIMIT_MS)).await;
        }

        print!("[{}/{}] {} ... ", i + 1, projects.len(), project);

        match fetch_repology_project(&client, project).await {
            Ok(packages) => {
                if packages.is_empty() {
                    println!("not found");
                    not_found.push(project.clone());
                    continue;
                }

                let mappings = map_to_sources(packages);
                println!("found in {} sources", mappings.len());

                if !is_batch && !is_crossref {
                    display_mappings(project, &mappings);
                }

                if do_update && !mappings.is_empty() {
                    let updated = update_source_files(&sources_dir, project, &mappings)?;
                    total_updated += updated.len();
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
            }
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("Summary:");
    println!("  Processed: {} packages", projects.len());
    if !not_found.is_empty() {
        println!("  Not found: {} ({:?})", not_found.len(), not_found);
    }
    if do_update {
        println!("  Files updated: {}", total_updated);
        if total_updated > 0 {
            println!("\nRun 'just generate-index' to regenerate known_packages.ccl");
        }
    }

    Ok(())
}

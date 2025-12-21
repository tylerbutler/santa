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
use serde::Deserialize;
use std::collections::BTreeMap;
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
            "  {} --from-crossref 50 --update",
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
    let do_update = args.iter().any(|a| a == "--update");

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

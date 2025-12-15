//! Cross-reference packages across collected sources and score by popularity.
//!
//! Builds a unified index of packages from all collected data sources,
//! scores them by popularity and curation presence, and outputs ranked results.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Package data from collected JSON files
#[derive(Debug, Deserialize)]
struct CollectedPackage {
    name: String,
    display_name: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    category: Option<String>,
    popularity: Option<i64>,
    popularity_rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CollectionResult {
    packages: Vec<CollectedPackage>,
}

/// Cross-referenced package with data from multiple sources
#[derive(Debug, Default)]
struct CrossRefPackage {
    name: String,
    display_name: String,
    description: Option<String>,
    homepage: Option<String>,
    category: Option<String>,

    in_homebrew: bool,
    in_toolleeo: bool,
    in_modern_unix: bool,
    in_awesome_cli: bool,

    homebrew_rank: Option<i32>,
    homebrew_installs: Option<i64>,

    score: i32,
    sources: Vec<String>,
}

/// Output format for crossref results
#[derive(Debug, Serialize)]
struct CrossRefOutput {
    generated_at: String,
    total_indexed: usize,
    existing_in_ccl: usize,
    packages: Vec<CrossRefResult>,
}

#[derive(Debug, Serialize)]
struct CrossRefResult {
    rank: usize,
    name: String,
    display_name: String,
    description: Option<String>,
    homepage: Option<String>,
    score: i32,
    in_homebrew: bool,
    in_modern_unix: bool,
    in_toolleeo: bool,
    in_awesome_cli: bool,
    homebrew_rank: Option<i32>,
    homebrew_installs: Option<i64>,
    category: Option<String>,
    sources: Vec<String>,
}

/// Normalize a package name for matching
fn normalize_name(name: &str) -> String {
    let mut name = name.to_lowercase();

    // Handle npm scoped packages
    if name.starts_with('@') && name.contains('/') {
        name = name.split('/').last().unwrap_or(&name).to_string();
    }

    // Strip common suffixes
    for suffix in &["-cli", "-bin", "-git", "-rs", "-go", "-rust"] {
        if name.ends_with(suffix) {
            name = name[..name.len() - suffix.len()].to_string();
            break;
        }
    }

    // Handle version suffixes like python@3.13
    if let Some(idx) = name.find('@') {
        name = name[..idx].to_string();
    }

    name
}

fn load_json_packages(path: &Path) -> Result<Vec<CollectedPackage>> {
    if !path.exists() {
        eprintln!("Warning: {} not found", path.display());
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let result: CollectionResult = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    Ok(result.packages)
}

fn build_crossref_index(data_dir: &Path) -> Result<BTreeMap<String, CrossRefPackage>> {
    let mut index: BTreeMap<String, CrossRefPackage> = BTreeMap::new();

    // Load Homebrew (has popularity data)
    let homebrew_pkgs = load_json_packages(&data_dir.join("homebrew.json"))?;
    println!("Loaded {} packages from Homebrew", homebrew_pkgs.len());

    for pkg in homebrew_pkgs {
        let norm_name = normalize_name(&pkg.name);
        let entry = index
            .entry(norm_name.clone())
            .or_insert_with(|| CrossRefPackage {
                name: norm_name.clone(),
                display_name: pkg.display_name.clone().unwrap_or_else(|| pkg.name.clone()),
                ..Default::default()
            });

        entry.in_homebrew = true;
        entry.homebrew_rank = pkg.popularity_rank;
        entry.homebrew_installs = pkg.popularity;
        entry.sources.push("homebrew".to_string());

        if entry.display_name.is_empty() {
            entry.display_name = pkg.display_name.unwrap_or_else(|| pkg.name.clone());
        }
        if entry.description.is_none() {
            entry.description = pkg.description;
        }
    }

    // Load Modern Unix
    let modern_unix_pkgs = load_json_packages(&data_dir.join("modern_unix.json"))?;
    println!(
        "Loaded {} packages from modern-unix",
        modern_unix_pkgs.len()
    );

    for pkg in modern_unix_pkgs {
        let norm_name = normalize_name(&pkg.name);
        let entry = index
            .entry(norm_name.clone())
            .or_insert_with(|| CrossRefPackage {
                name: norm_name.clone(),
                display_name: pkg.display_name.clone().unwrap_or_else(|| pkg.name.clone()),
                ..Default::default()
            });

        entry.in_modern_unix = true;
        entry.sources.push("modern_unix".to_string());

        if entry.description.is_none() {
            entry.description = pkg.description;
        }
        if entry.homepage.is_none() {
            entry.homepage = pkg.homepage;
        }
    }

    // Load Toolleeo
    let toolleeo_pkgs = load_json_packages(&data_dir.join("toolleeo.json"))?;
    println!("Loaded {} packages from toolleeo", toolleeo_pkgs.len());

    for pkg in toolleeo_pkgs {
        let norm_name = normalize_name(&pkg.name);
        let entry = index
            .entry(norm_name.clone())
            .or_insert_with(|| CrossRefPackage {
                name: norm_name.clone(),
                display_name: pkg.display_name.clone().unwrap_or_else(|| pkg.name.clone()),
                ..Default::default()
            });

        entry.in_toolleeo = true;
        entry.sources.push("toolleeo".to_string());

        if entry.category.is_none() {
            entry.category = pkg.category;
        }
        if entry.description.is_none() {
            entry.description = pkg.description;
        }
    }

    // Load Awesome CLI Apps
    let awesome_pkgs = load_json_packages(&data_dir.join("awesome_cli_apps.json"))?;
    println!(
        "Loaded {} packages from awesome-cli-apps",
        awesome_pkgs.len()
    );

    for pkg in awesome_pkgs {
        let norm_name = normalize_name(&pkg.name);
        let entry = index
            .entry(norm_name.clone())
            .or_insert_with(|| CrossRefPackage {
                name: norm_name.clone(),
                display_name: pkg.display_name.clone().unwrap_or_else(|| pkg.name.clone()),
                ..Default::default()
            });

        entry.in_awesome_cli = true;
        entry.sources.push("awesome_cli_apps".to_string());

        if entry.category.is_none() {
            entry.category = pkg.category;
        }
        if entry.description.is_none() {
            entry.description = pkg.description;
        }
    }

    Ok(index)
}

fn score_packages(index: &mut BTreeMap<String, CrossRefPackage>) {
    for pkg in index.values_mut() {
        let mut score = 0i32;

        // Homebrew rank (most weight - has real usage data)
        if let Some(rank) = pkg.homebrew_rank {
            score += (501 - rank).max(0);
        }

        // Curated list presence
        if pkg.in_modern_unix {
            score += 200;
        }
        if pkg.in_toolleeo {
            score += 50;
        }
        if pkg.in_awesome_cli {
            score += 50;
        }

        // Bonus for multiple sources
        let source_count = pkg.sources.iter().collect::<BTreeSet<_>>().len();
        if source_count >= 3 {
            score += 100;
        } else if source_count >= 2 {
            score += 50;
        }

        pkg.score = score;
    }
}

fn load_existing_packages(ccl_path: &Path) -> Result<BTreeSet<String>> {
    if !ccl_path.exists() {
        return Ok(BTreeSet::new());
    }

    let content = fs::read_to_string(ccl_path)?;
    let mut existing = BTreeSet::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("/=") || line.starts_with('=') {
            continue;
        }
        if line.contains('=') && !line.starts_with(' ') {
            if let Some(name) = line.split('=').next() {
                let name = name.trim();
                if !name.is_empty() {
                    existing.insert(normalize_name(name));
                }
            }
        }
    }

    Ok(existing)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let discovery_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("discovery");

    let data_dir = discovery_dir.join("raw");
    let ccl_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("known_packages.ccl");
    let output_path = discovery_dir.join("crossref_results.json");

    let top: usize = args
        .get(1)
        .and_then(|s| s.strip_prefix("--top="))
        .or_else(|| args.get(2).map(|s| s.as_str()))
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);

    let include_existing = args.iter().any(|a| a == "--include-existing");

    println!(
        "\nBuilding cross-reference index from {}...",
        data_dir.display()
    );
    let mut index = build_crossref_index(&data_dir)?;
    println!("Total unique packages indexed: {}", index.len());

    println!("\nScoring packages...");
    score_packages(&mut index);

    let existing = load_existing_packages(&ccl_path)?;
    println!("Existing packages in CCL: {}", existing.len());

    // Sort by score
    let mut ranked: Vec<_> = index.values().collect();
    ranked.sort_by(|a, b| b.score.cmp(&a.score));

    // Filter existing if needed
    if !include_existing {
        ranked.retain(|p| !existing.contains(&p.name));
        println!("After filtering existing: {} candidates", ranked.len());
    }

    // Take top N
    let top_packages: Vec<_> = ranked.into_iter().take(top).collect();

    println!("\nTop {} packages by score:", top_packages.len());
    println!("{}", "-".repeat(80));

    let mut results = Vec::new();
    for (i, pkg) in top_packages.iter().enumerate() {
        let sources: BTreeSet<_> = pkg.sources.iter().collect();
        let sources_str = sources
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{:3}. {:<25} score={:4} sources=[{}]",
            i + 1,
            pkg.name,
            pkg.score,
            sources_str
        );

        results.push(CrossRefResult {
            rank: i + 1,
            name: pkg.name.clone(),
            display_name: pkg.display_name.clone(),
            description: pkg.description.clone(),
            homepage: pkg.homepage.clone(),
            score: pkg.score,
            in_homebrew: pkg.in_homebrew,
            in_modern_unix: pkg.in_modern_unix,
            in_toolleeo: pkg.in_toolleeo,
            in_awesome_cli: pkg.in_awesome_cli,
            homebrew_rank: pkg.homebrew_rank,
            homebrew_installs: pkg.homebrew_installs,
            category: pkg.category.clone(),
            sources: pkg
                .sources
                .iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .cloned()
                .collect(),
        });
    }

    // Save results
    let output = CrossRefOutput {
        generated_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        total_indexed: index.len(),
        existing_in_ccl: existing.len(),
        packages: results,
    };

    fs::create_dir_all(output_path.parent().unwrap())?;
    fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
    println!("\nResults saved to {}", output_path.display());

    // Summary
    println!("\n{}", "=".repeat(80));
    println!("SUMMARY");
    println!("{}", "=".repeat(80));

    let in_homebrew = top_packages.iter().filter(|p| p.in_homebrew).count();
    let in_modern = top_packages.iter().filter(|p| p.in_modern_unix).count();
    let in_toolleeo = top_packages.iter().filter(|p| p.in_toolleeo).count();
    let in_awesome = top_packages.iter().filter(|p| p.in_awesome_cli).count();

    println!("Packages in Homebrew:      {}", in_homebrew);
    println!("Packages in modern-unix:   {}", in_modern);
    println!("Packages in toolleeo:      {}", in_toolleeo);
    println!("Packages in awesome-cli:   {}", in_awesome);

    Ok(())
}

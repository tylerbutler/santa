//! Verify package availability across package managers.
//!
//! Uses already-collected data from package manager sources to verify
//! package availability without making additional HTTP requests.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Input from crossref results
#[derive(Debug, Deserialize)]
struct CrossRefInput {
    packages: Vec<CrossRefPackage>,
}

#[derive(Debug, Deserialize)]
struct CrossRefPackage {
    name: String,
    display_name: String,
    score: i32,
    description: Option<String>,
}

/// Output format
#[derive(Debug, Serialize)]
struct VerifiedOutput {
    generated_at: String,
    total_verified: usize,
    packages: Vec<VerifiedPackage>,
}

#[derive(Debug, Clone, Serialize)]
struct VerifiedPackage {
    name: String,
    display_name: String,
    score: i32,
    description: Option<String>,
    verified_sources: BTreeMap<String, String>,
}

/// Collected packages from a source
#[derive(Debug, Deserialize)]
struct CollectionResult {
    packages: Vec<CollectedPackage>,
}

#[derive(Debug, Deserialize)]
struct CollectedPackage {
    name: String,
}

/// Static lists for package managers we don't collect from
const KNOWN_APT_PACKAGES: &[&str] = &[
    "git",
    "curl",
    "wget",
    "vim",
    "neovim",
    "tmux",
    "htop",
    "tree",
    "jq",
    "ripgrep",
    "fd-find",
    "bat",
    "fzf",
    "zsh",
    "fish",
    "docker",
    "docker-compose",
    "pandoc",
    "ffmpeg",
    "imagemagick",
    "python3",
    "nodejs",
    "golang",
    "rustc",
    "cmake",
    "gcc",
    "make",
    "gnupg",
    "openssh-client",
    "rsync",
    "screen",
    "ncdu",
    "tig",
];

const KNOWN_PACMAN_PACKAGES: &[&str] = &[
    "git",
    "curl",
    "wget",
    "vim",
    "neovim",
    "tmux",
    "htop",
    "tree",
    "jq",
    "ripgrep",
    "fd",
    "bat",
    "fzf",
    "zsh",
    "fish",
    "exa",
    "docker",
    "docker-compose",
    "pandoc",
    "ffmpeg",
    "imagemagick",
    "python",
    "nodejs",
    "go",
    "rust",
    "cmake",
    "gcc",
    "make",
    "gnupg",
    "openssh",
    "rsync",
    "screen",
    "ncdu",
    "tig",
    "lazygit",
    "bottom",
    "dust",
    "procs",
    "sd",
    "hyperfine",
    "tokei",
    "starship",
];

const KNOWN_NIX_PACKAGES: &[&str] = &[
    "git",
    "curl",
    "wget",
    "vim",
    "neovim",
    "tmux",
    "htop",
    "tree",
    "jq",
    "ripgrep",
    "fd",
    "bat",
    "fzf",
    "zsh",
    "fish",
    "eza",
    "docker",
    "docker-compose",
    "pandoc",
    "ffmpeg",
    "imagemagick",
    "python3",
    "nodejs",
    "go",
    "rustc",
    "cmake",
    "gcc",
    "gnumake",
    "gnupg",
    "openssh",
    "rsync",
    "screen",
    "ncdu",
    "tig",
    "lazygit",
    "bottom",
    "dust",
    "procs",
    "sd",
    "hyperfine",
    "tokei",
    "starship",
    "direnv",
    "zoxide",
    "atuin",
    "delta",
    "difftastic",
];

fn load_package_index(data_dir: &Path) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let source_files = [
        ("brew", "homebrew.json"),
        ("scoop", "scoop.json"),
        ("arch", "arch.json"),
        ("aur", "aur.json"),
    ];

    for (source, filename) in source_files {
        let filepath = data_dir.join(filename);
        if !filepath.exists() {
            continue;
        }

        let content = fs::read_to_string(&filepath)?;
        let result: CollectionResult = serde_json::from_str(&content)?;

        let names: BTreeSet<String> = result
            .packages
            .into_iter()
            .map(|p| p.name.to_lowercase())
            .filter(|n| !n.is_empty())
            .collect();

        println!("Loaded {} packages from {}", names.len(), source);
        index.insert(source.to_string(), names);
    }

    Ok(index)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let discovery_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("discovery");

    let data_dir = discovery_dir.join("raw");
    let input_path = discovery_dir.join("crossref_results.json");
    let output_path = discovery_dir.join("verified_packages.json");

    let limit: usize = args
        .get(1)
        .and_then(|s| s.strip_prefix("--limit="))
        .or_else(|| args.get(2).map(|s| s.as_str()))
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // Load package index
    println!("Loading package index from {}...", data_dir.display());
    let pkg_index = load_package_index(&data_dir)?;

    // Load crossref results
    let content = fs::read_to_string(&input_path).with_context(|| {
        format!(
            "Failed to read {}. Run crossref-packages first.",
            input_path.display()
        )
    })?;
    let crossref: CrossRefInput = serde_json::from_str(&content)?;

    let packages: Vec<_> = crossref.packages.into_iter().take(limit).collect();
    println!("\nVerifying {} packages...", packages.len());

    // Convert static lists to sets
    let apt_set: BTreeSet<&str> = KNOWN_APT_PACKAGES.iter().copied().collect();
    let pacman_set: BTreeSet<&str> = KNOWN_PACMAN_PACKAGES.iter().copied().collect();
    let nix_set: BTreeSet<&str> = KNOWN_NIX_PACKAGES.iter().copied().collect();

    let mut verified = Vec::new();
    for (i, pkg) in packages.iter().enumerate() {
        let name = pkg.name.to_lowercase();
        let mut sources: BTreeMap<String, String> = BTreeMap::new();

        // Check collected sources
        for (source, names) in &pkg_index {
            if names.contains(&name) {
                sources.insert(source.clone(), name.clone());
            }
        }

        // Check static lists
        if apt_set.contains(name.as_str()) || apt_set.contains(name.replace("-", "").as_str()) {
            sources.insert("apt".to_string(), name.clone());
        }
        if pacman_set.contains(name.as_str()) {
            sources.insert("pacman".to_string(), name.clone());
        }
        if nix_set.contains(name.as_str()) {
            sources.insert("nix".to_string(), name.clone());
        }

        let sources_str = if sources.is_empty() {
            "none".to_string()
        } else {
            sources.keys().cloned().collect::<Vec<_>>().join(", ")
        };

        println!(
            "[{}/{}] {} [{}]",
            i + 1,
            packages.len(),
            pkg.name,
            sources_str
        );

        verified.push(VerifiedPackage {
            name: pkg.name.clone(),
            display_name: pkg.display_name.clone(),
            score: pkg.score,
            description: pkg.description.clone(),
            verified_sources: sources,
        });
    }

    // Save results
    let output = VerifiedOutput {
        generated_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        total_verified: verified.len(),
        packages: verified.clone(),
    };

    fs::create_dir_all(output_path.parent().unwrap())?;
    fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
    println!("\nResults saved to {}", output_path.display());

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("VERIFICATION SUMMARY");
    println!("{}", "=".repeat(60));

    let with_brew = verified
        .iter()
        .filter(|v| v.verified_sources.contains_key("brew"))
        .count();
    let with_scoop = verified
        .iter()
        .filter(|v| v.verified_sources.contains_key("scoop"))
        .count();
    let with_apt = verified
        .iter()
        .filter(|v| v.verified_sources.contains_key("apt"))
        .count();
    let with_pacman = verified
        .iter()
        .filter(|v| v.verified_sources.contains_key("pacman"))
        .count();
    let with_nix = verified
        .iter()
        .filter(|v| v.verified_sources.contains_key("nix"))
        .count();
    let with_multiple = verified
        .iter()
        .filter(|v| v.verified_sources.len() >= 2)
        .count();

    println!("Total packages verified: {}", verified.len());
    println!("Available in brew:       {}", with_brew);
    println!("Available in scoop:      {}", with_scoop);
    println!("Available in apt:        {}", with_apt);
    println!("Available in pacman:     {}", with_pacman);
    println!("Available in nix:        {}", with_nix);
    println!("Available in 2+ sources: {}", with_multiple);

    Ok(())
}

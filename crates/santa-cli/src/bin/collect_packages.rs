//! Collect packages from various sources.
//!
//! Fetches package data from Homebrew, Scoop, AUR, Arch, and curated lists.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Package data to collect
#[derive(Debug, Serialize, Clone)]
struct Package {
    name: String,
    display_name: Option<String>,
    source: String,
    source_id: String,
    popularity: Option<i64>,
    popularity_rank: Option<i32>,
    description: Option<String>,
    homepage: Option<String>,
    category: Option<String>,
    collected_at: String,
}

#[derive(Debug, Serialize)]
struct CollectionResult {
    source: String,
    packages: Vec<Package>,
    collected_at: String,
    total_count: usize,
    errors: Vec<String>,
}

/// Homebrew analytics response
#[derive(Debug, Deserialize)]
struct HomebrewAnalytics {
    items: Vec<HomebrewItem>,
}

#[derive(Debug, Deserialize)]
struct HomebrewItem {
    formula: String,
    count: String,
    number: i32,
}

/// AUR RPC response
#[derive(Debug, Deserialize)]
struct AurResponse {
    results: Vec<AurPackage>,
}

#[derive(Debug, Deserialize)]
struct AurPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "NumVotes")]
    num_votes: Option<i64>,
}

/// Arch package list response
#[derive(Debug, Deserialize)]
struct ArchResponse {
    results: Vec<ArchPackage>,
}

#[derive(Debug, Deserialize)]
struct ArchPackage {
    pkgname: String,
    pkgdesc: Option<String>,
    url: Option<String>,
}

struct Collector {
    client: Client,
    output_dir: PathBuf,
    errors: Vec<String>,
}

impl Collector {
    fn new(output_dir: PathBuf) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("santa-package-collector/0.1")
            .build()?;

        Ok(Self {
            client,
            output_dir,
            errors: Vec::new(),
        })
    }

    fn today() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    async fn collect_homebrew(&mut self, limit: usize) -> Result<Vec<Package>> {
        println!("Fetching Homebrew analytics...");
        let url = "https://formulae.brew.sh/api/analytics/install-on-request/365d.json";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch Homebrew analytics")?;

        let data: HomebrewAnalytics = response
            .json()
            .await
            .context("Failed to parse Homebrew response")?;

        let packages: Vec<Package> = data
            .items
            .into_iter()
            .take(limit)
            .map(|item| {
                let count: i64 = item.count.replace(',', "").parse().unwrap_or(0);
                Package {
                    name: item.formula.to_lowercase(),
                    display_name: Some(item.formula.clone()),
                    source: "homebrew".to_string(),
                    source_id: item.formula,
                    popularity: Some(count),
                    popularity_rank: Some(item.number),
                    description: None,
                    homepage: None,
                    category: None,
                    collected_at: Self::today(),
                }
            })
            .collect();

        println!("Collected {} packages from Homebrew", packages.len());
        Ok(packages)
    }

    async fn collect_scoop(&mut self) -> Result<Vec<Package>> {
        println!("Fetching Scoop main bucket...");

        // Get directory listing via GitHub API
        let api_url = "https://api.github.com/repos/ScoopInstaller/Main/contents/bucket";

        let response = self
            .client
            .get(api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch Scoop bucket listing")?;

        #[derive(Deserialize)]
        struct GhFile {
            name: String,
        }

        let files: Vec<GhFile> = response
            .json()
            .await
            .context("Failed to parse Scoop directory listing")?;

        let packages: Vec<Package> = files
            .into_iter()
            .filter(|f| f.name.ends_with(".json"))
            .map(|f| {
                let name = f.name.trim_end_matches(".json").to_lowercase();
                Package {
                    name: name.clone(),
                    display_name: Some(f.name.trim_end_matches(".json").to_string()),
                    source: "scoop".to_string(),
                    source_id: name.clone(),
                    popularity: None,
                    popularity_rank: None,
                    description: None,
                    homepage: None,
                    category: None,
                    collected_at: Self::today(),
                }
            })
            .collect();

        println!("Collected {} packages from Scoop", packages.len());
        Ok(packages)
    }

    async fn collect_aur(&mut self, limit: usize) -> Result<Vec<Package>> {
        println!("Fetching AUR packages (by votes)...");

        // This is a workaround - AUR search is limited
        // For a real implementation, we'd need to paginate or use a different approach
        let common_prefixes = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "r",
            "s", "t", "u", "v", "w", "x", "y", "z",
        ];

        let mut all_packages: BTreeSet<String> = BTreeSet::new();
        let mut packages = Vec::new();

        // Just get a sample using a few prefix searches
        for prefix in &common_prefixes[..5] {
            let url = format!(
                "https://aur.archlinux.org/rpc/?v=5&type=search&by=name&arg={}",
                prefix
            );

            match self.client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<AurResponse>().await {
                        for pkg in data.results {
                            if !all_packages.contains(&pkg.name) {
                                all_packages.insert(pkg.name.clone());
                                packages.push(Package {
                                    name: pkg.name.to_lowercase(),
                                    display_name: Some(pkg.name.clone()),
                                    source: "aur".to_string(),
                                    source_id: pkg.name,
                                    popularity: pkg.num_votes,
                                    popularity_rank: None,
                                    description: pkg.description,
                                    homepage: pkg.url,
                                    category: None,
                                    collected_at: Self::today(),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    self.errors
                        .push(format!("AUR search for '{}': {}", prefix, e));
                }
            }

            if packages.len() >= limit {
                break;
            }
        }

        // Sort by votes and limit
        packages.sort_by(|a, b| b.popularity.cmp(&a.popularity));
        packages.truncate(limit);

        println!("Collected {} packages from AUR", packages.len());
        Ok(packages)
    }

    async fn collect_arch(&mut self, limit: usize) -> Result<Vec<Package>> {
        println!("Fetching Arch official packages...");

        let repos = ["core", "extra"];
        let mut packages = Vec::new();

        for repo in repos {
            let url = format!(
                "https://archlinux.org/packages/search/json/?repo={}&arch=x86_64",
                repo
            );

            match self.client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(data) = response.json::<ArchResponse>().await {
                        for pkg in data.results {
                            packages.push(Package {
                                name: pkg.pkgname.to_lowercase(),
                                display_name: Some(pkg.pkgname.clone()),
                                source: "arch".to_string(),
                                source_id: pkg.pkgname,
                                popularity: None,
                                popularity_rank: None,
                                description: pkg.pkgdesc,
                                homepage: pkg.url,
                                category: None,
                                collected_at: Self::today(),
                            });
                        }
                    }
                }
                Err(e) => {
                    self.errors.push(format!("Arch {} fetch: {}", repo, e));
                }
            }

            if packages.len() >= limit {
                break;
            }
        }

        packages.truncate(limit);
        println!("Collected {} packages from Arch", packages.len());
        Ok(packages)
    }

    async fn collect_modern_unix(&mut self) -> Result<Vec<Package>> {
        println!("Fetching modern-unix list...");
        let url = "https://raw.githubusercontent.com/ibraheemdev/modern-unix/master/readme.md";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch modern-unix")?;

        let content = response.text().await?;

        // Parse markdown to extract tool names
        let mut packages = Vec::new();
        for line in content.lines() {
            // Look for markdown links like [tool](url)
            if line.starts_with("* [") || line.starts_with("- [") {
                if let Some(start) = line.find('[') {
                    if let Some(end) = line[start..].find(']') {
                        let name = &line[start + 1..start + end];
                        // Extract URL if present
                        let homepage = if let Some(url_start) = line.find("](") {
                            if let Some(url_end) = line[url_start + 2..].find(')') {
                                Some(line[url_start + 2..url_start + 2 + url_end].to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        packages.push(Package {
                            name: name.to_lowercase(),
                            display_name: Some(name.to_string()),
                            source: "modern_unix".to_string(),
                            source_id: name.to_lowercase(),
                            popularity: None,
                            popularity_rank: None,
                            description: None,
                            homepage,
                            category: None,
                            collected_at: Self::today(),
                        });
                    }
                }
            }
        }

        println!("Collected {} packages from modern-unix", packages.len());
        Ok(packages)
    }

    async fn collect_toolleeo(&mut self) -> Result<Vec<Package>> {
        println!("Fetching toolleeo CLI apps list...");
        let url = "https://raw.githubusercontent.com/toolleeo/cli-apps/master/data/cli-apps.csv";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch toolleeo")?;

        let content = response.text().await?;

        let mut packages = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if i == 0 {
                continue; // Skip header
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim().trim_matches('"');
                let description = if parts.len() > 1 {
                    Some(parts[1].trim().trim_matches('"').to_string())
                } else {
                    None
                };
                let category = if parts.len() > 2 {
                    Some(parts[2].trim().trim_matches('"').to_string())
                } else {
                    None
                };

                if !name.is_empty() {
                    packages.push(Package {
                        name: name.to_lowercase(),
                        display_name: Some(name.to_string()),
                        source: "toolleeo".to_string(),
                        source_id: name.to_lowercase(),
                        popularity: None,
                        popularity_rank: None,
                        description,
                        homepage: None,
                        category,
                        collected_at: Self::today(),
                    });
                }
            }
        }

        println!("Collected {} packages from toolleeo", packages.len());
        Ok(packages)
    }

    async fn collect_awesome_cli(&mut self) -> Result<Vec<Package>> {
        println!("Fetching awesome-cli-apps list...");
        let url = "https://raw.githubusercontent.com/agarrharr/awesome-cli-apps/main/readme.md";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch awesome-cli-apps")?;

        let content = response.text().await?;

        let mut packages = Vec::new();
        let mut current_category = String::new();

        for line in content.lines() {
            // Track category headers
            if line.starts_with("## ") {
                current_category = line[3..].trim().to_string();
                continue;
            }

            // Look for list items with links
            if (line.starts_with("- [") || line.starts_with("* [")) && line.contains("](") {
                if let Some(start) = line.find('[') {
                    if let Some(end) = line[start..].find(']') {
                        let name = &line[start + 1..start + end];

                        let homepage = if let Some(url_start) = line.find("](") {
                            if let Some(url_end) = line[url_start + 2..].find(')') {
                                Some(line[url_start + 2..url_start + 2 + url_end].to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Extract description (text after the link)
                        let description = if let Some(desc_start) = line.find(") - ") {
                            Some(line[desc_start + 4..].trim().to_string())
                        } else {
                            None
                        };

                        packages.push(Package {
                            name: name.to_lowercase(),
                            display_name: Some(name.to_string()),
                            source: "awesome_cli_apps".to_string(),
                            source_id: name.to_lowercase(),
                            popularity: None,
                            popularity_rank: None,
                            description,
                            homepage,
                            category: if current_category.is_empty() {
                                None
                            } else {
                                Some(current_category.clone())
                            },
                            collected_at: Self::today(),
                        });
                    }
                }
            }
        }

        println!(
            "Collected {} packages from awesome-cli-apps",
            packages.len()
        );
        Ok(packages)
    }

    fn save(&self, source: &str, packages: Vec<Package>) -> Result<PathBuf> {
        fs::create_dir_all(&self.output_dir)?;

        let output_path = self.output_dir.join(format!("{}.json", source));

        let result = CollectionResult {
            source: source.to_string(),
            packages: packages.clone(),
            collected_at: Self::today(),
            total_count: packages.len(),
            errors: self.errors.clone(),
        };

        fs::write(&output_path, serde_json::to_string_pretty(&result)?)?;
        Ok(output_path)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("discovery");

    let output_dir = data_dir.join("raw");

    // Parse arguments
    let homebrew_limit: usize = args
        .iter()
        .find(|a| a.starts_with("--homebrew-limit="))
        .and_then(|a| a.strip_prefix("--homebrew-limit="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let arch_limit: usize = args
        .iter()
        .find(|a| a.starts_with("--arch-limit="))
        .and_then(|a| a.strip_prefix("--arch-limit="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);

    let sources: Vec<&str> = if args.iter().any(|a| a == "--list") {
        println!("Available collectors:");
        println!("  - homebrew");
        println!("  - scoop");
        println!("  - aur");
        println!("  - arch");
        println!("  - modern_unix");
        println!("  - toolleeo");
        println!("  - awesome_cli_apps");
        return Ok(());
    } else if let Some(pos) = args.iter().position(|a| a == "--sources") {
        args.iter()
            .skip(pos + 1)
            .take_while(|a| !a.starts_with("--"))
            .map(|s| s.as_str())
            .collect()
    } else {
        vec![
            "homebrew",
            "scoop",
            "aur",
            "arch",
            "modern_unix",
            "toolleeo",
            "awesome_cli_apps",
        ]
    };

    println!("Running collectors: {}\n", sources.join(", "));

    let mut collector = Collector::new(output_dir)?;
    let mut total = 0;

    for source in &sources {
        let result = match *source {
            "homebrew" => collector.collect_homebrew(homebrew_limit).await,
            "scoop" => collector.collect_scoop().await,
            "aur" => collector.collect_aur(500).await,
            "arch" => collector.collect_arch(arch_limit).await,
            "modern_unix" => collector.collect_modern_unix().await,
            "toolleeo" => collector.collect_toolleeo().await,
            "awesome_cli_apps" => collector.collect_awesome_cli().await,
            _ => {
                eprintln!("Unknown source: {}", source);
                continue;
            }
        };

        match result {
            Ok(packages) => {
                let count = packages.len();
                let path = collector.save(source, packages)?;
                println!("  Saved to {}", path.display());
                total += count;
            }
            Err(e) => {
                eprintln!("  Error collecting {}: {}", source, e);
            }
        }
        println!();
    }

    println!("{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Total packages collected: {}", total);

    Ok(())
}

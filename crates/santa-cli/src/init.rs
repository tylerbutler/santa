use crate::data::SantaData;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};
use std::path::{Path, PathBuf};

/// Known Tier 1 managers and the binary name to detect them
const DETECTABLE_MANAGERS: &[(&str, &str)] = &[
    ("brew", "brew"),
    ("apt", "apt"),
    ("pacman", "pacman"),
    ("cargo", "cargo"),
    ("npm", "npm"),
];

const STARTER_PACKAGES: &[&str] = &["ripgrep", "fd", "jq", "bat", "eza"];

/// Detect which package managers are available on $PATH
fn detect_managers() -> Vec<String> {
    DETECTABLE_MANAGERS
        .iter()
        .filter(|(_, bin)| which::which(bin).is_ok())
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Get the default config path
fn default_config_path() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().context("Failed to get base directories")?;
    Ok(base.home_dir().join(".config/santa/config.ccl"))
}

fn starter_packages_for(managers: &[String], data: &SantaData) -> Vec<&'static str> {
    STARTER_PACKAGES
        .iter()
        .copied()
        .filter(|package| {
            data.packages.get(*package).is_some_and(|sources| {
                managers
                    .iter()
                    .any(|selected| sources.keys().any(|source| source.to_string() == *selected))
            })
        })
        .collect()
}

/// Generate CCL config content for the selected managers and packages.
fn generate_config(managers: &[String], include_starter: bool, data: &SantaData) -> String {
    let mut lines = vec!["/= Santa package configuration".to_string(), String::new()];

    lines.push("sources =".to_string());
    for manager in managers {
        lines.push(format!("  = {manager}"));
    }
    lines.push(String::new());

    lines.push("packages =".to_string());
    if include_starter {
        for package in starter_packages_for(managers, data) {
            lines.push(format!("  = {package}"));
        }
    }

    lines.push(String::new());

    lines.join("\n")
}

/// Run the init command
pub async fn run_init(yes: bool, output: Option<&Path>) -> Result<()> {
    let config_path = match output {
        Some(p) => p.to_path_buf(),
        None => default_config_path()?,
    };

    // Check for existing config
    if config_path.exists() {
        if yes {
            bail!(
                "Config already exists at {}. Remove it first or use --output to write elsewhere.",
                config_path.display()
            );
        }
        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Config already exists at {}. Overwrite?",
                config_path.display()
            ))
            .default(false)
            .interact()?;
        if !overwrite {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Detect available managers
    let detected = detect_managers();
    println!(
        "Detected package managers: {}",
        if detected.is_empty() {
            "none".dimmed().to_string()
        } else {
            detected.join(", ")
        }
    );

    // Select managers
    let selected = if yes {
        detected.clone()
    } else {
        let all_managers: Vec<&str> = DETECTABLE_MANAGERS.iter().map(|(name, _)| *name).collect();
        let defaults: Vec<bool> = all_managers
            .iter()
            .map(|m| detected.contains(&m.to_string()))
            .collect();

        let chosen_indices = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select package managers to include")
            .items(&all_managers)
            .defaults(&defaults)
            .interact()?;

        chosen_indices
            .into_iter()
            .map(|i| all_managers[i].to_string())
            .collect()
    };

    if selected.is_empty() {
        bail!("No package managers selected. Config not created.");
    }

    let data = SantaData::default();

    // Ask about starter packages
    let include_starter = if yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include starter packages (ripgrep, fd, jq, bat, eza when supported)?")
            .default(true)
            .interact()?
    };

    // Generate and write config
    let content = generate_config(&selected, include_starter, &data);

    // Create parent directories
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    std::fs::write(&config_path, &content)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

    println!(
        "\n{} Config written to {}",
        "✓".green(),
        config_path.display()
    );
    println!("\nNext steps:");
    println!("  santa status    — see what's installed vs. missing");
    println!("  santa install   — generate install scripts");
    println!("  santa add <pkg> — add more packages");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::starter_packages_for;
    use crate::data::SantaData;

    #[test]
    fn starter_packages_only_include_supported_packages_for_selected_sources() {
        let data = SantaData::default();

        let apt_only = starter_packages_for(&["apt".to_string()], &data);
        assert!(!apt_only.contains(&"fd"));
        assert!(apt_only.contains(&"ripgrep"));
        assert!(apt_only.contains(&"jq"));
        assert!(apt_only.contains(&"bat"));

        let cargo_only = starter_packages_for(&["cargo".to_string()], &data);
        assert!(cargo_only.is_empty());

        let brew_only = starter_packages_for(&["brew".to_string()], &data);
        assert!(brew_only.contains(&"eza"));

        for package in &brew_only {
            let supported = data
                .packages
                .get(*package)
                .is_some_and(|sources| sources.keys().any(|source| source.to_string() == "brew"));
            assert!(supported, "{package} should be installable with brew");
        }
    }
}

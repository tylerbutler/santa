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

/// Starter packages per manager (canonical name -> manager-specific name)
const STARTER_PACKAGES: &[(&str, &[(&str, &str)])] = &[
    ("brew", &[("ripgrep", "ripgrep"), ("fd", "fd"), ("jq", "jq"), ("bat", "bat")]),
    ("apt", &[("ripgrep", "ripgrep"), ("fd", "fd-find"), ("jq", "jq"), ("bat", "bat")]),
    ("pacman", &[("ripgrep", "ripgrep"), ("fd", "fd"), ("jq", "jq"), ("bat", "bat")]),
    ("cargo", &[("bat", "bat"), ("eza", "eza"), ("ripgrep", "ripgrep")]),
    ("npm", &[]),
];

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

/// Generate CCL config content for the selected managers and packages
fn generate_config(managers: &[String], include_starter: bool) -> String {
    let mut lines = vec!["/= Santa package configuration".to_string(), String::new()];

    for manager in managers {
        lines.push(format!("{manager} ="));
        if include_starter {
            if let Some((_, packages)) = STARTER_PACKAGES.iter().find(|(m, _)| *m == manager.as_str()) {
                for (_, pkg_name) in *packages {
                    lines.push(format!("  = {pkg_name}"));
                }
            }
        }
        lines.push(String::new());
    }

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

    // Ask about starter packages
    let include_starter = if yes {
        true
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Include starter packages (ripgrep, fd, jq, bat)?")
            .default(true)
            .interact()?
    };

    // Generate and write config
    let content = generate_config(&selected, include_starter);

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

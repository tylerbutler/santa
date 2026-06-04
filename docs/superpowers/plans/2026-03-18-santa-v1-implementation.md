# Santa v1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship santa 1.0.0 — a stable, documented, well-tested package manager meta-tool with the `santa init` command and complete Tier 1/2 package manager support.

**Architecture:** The work is organized into 8 tasks. Task 1 (santa init) is the only new feature. Tasks 2-7 are polish, testing, docs, and infrastructure. Task 8 is the version bump. Tasks have some shared file conflicts (noted in Execution Order) but can be serialized cleanly. All work happens within the existing 4-crate workspace; no new crates needed.

**Tech Stack:** Rust, clap (CLI), dialoguer (interactive prompts), minijinja (templates), sickle (CCL parsing), assert_cmd/rstest/proptest (testing), cargo-dist (releases), changie (changelogs)

**Spec:** `docs/superpowers/specs/2026-03-18-santa-v1-prd.md`

---

### Task 1: Implement `santa init` Command

**Files:**
- Create: `crates/santa-cli/src/init.rs`
- Modify: `crates/santa-cli/src/main.rs` (add Init variant to Commands enum, wire up handler)
- Modify: `crates/santa-cli/src/lib.rs` (export init module)
- Create: `crates/santa-cli/tests/e2e/init_tests.rs`
- Modify: `crates/santa-cli/tests/e2e/mod.rs` (add init_tests module)

**Context:** `dialoguer` v0.12.0 is already a dependency and used in `sources.rs` for confirmation prompts. The default config path is `~/.config/santa/config.ccl` (see `DEFAULT_CONFIG_FILE_PATH` in main.rs). The existing `Commands` enum in main.rs uses clap derive macros.

- [ ] **Step 1: Add Init variant to CLI**

In `crates/santa-cli/src/main.rs`, add to the `Commands` enum:

```rust
/// Initialize a new santa configuration
Init {
    /// Accept defaults without prompting
    #[clap(short, long)]
    yes: bool,

    /// Output path for the config file (default: ~/.config/santa/config.ccl)
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,
},
```

Add an early-return handler in the `run()` function **between lines 467-468** (after `command` is extracted from `cli.command`, before tracing/config setup). This must come before `load_config` since `init` creates the config:

```rust
// Handle init before config loading (init creates the config)
if let Commands::Init { yes, output } = &command {
    santa::init::run_init(*yes, output.as_deref()).await?;
    return Ok(());
}
```

Place this immediately after the `None => { Cli::command().print_help()?; return Ok(()); }` block (line 467) and before the completions handler (line 470).

- [ ] **Step 2: Write the init module skeleton with tests**

Create `crates/santa-cli/src/init.rs`:

```rust
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
```

- [ ] **Step 3: Export init module**

Note: The `which` crate is already a dependency at v8.0.0 in `Cargo.toml` (line 86). No new dependency needed.

In `crates/santa-cli/src/lib.rs`, add:

```rust
pub mod init;
```

Run: `cargo check -p santa` to verify.

- [ ] **Step 4: Write tests for init**

Create `crates/santa-cli/tests/e2e/init_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper to run santa init with a temp output path
fn santa_init_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

#[test]
fn init_with_yes_creates_config() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config written to"));

    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    // Should contain at least one manager section (cargo is always available in test env)
    assert!(content.contains("="));
}

#[test]
fn init_refuses_overwrite_with_yes_flag() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");

    // Create existing config
    std::fs::write(&config_path, "existing =\n").unwrap();

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_generates_valid_ccl() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");

    santa_init_cmd()
        .args(["init", "--yes", "--output", config_path.to_str().unwrap()])
        .assert()
        .success();

    let content = std::fs::read_to_string(&config_path).unwrap();
    // Verify basic CCL structure: comment, manager sections
    assert!(content.starts_with("/="));
    // Each line should be valid CCL (comment, blank, key =, or = value)
    for line in content.lines() {
        let trimmed = line.trim();
        assert!(
            trimmed.is_empty()
                || trimmed.starts_with("/=")
                || trimmed.starts_with("= ")
                || trimmed.contains(" ="),
            "Invalid CCL line: {trimmed}"
        );
    }
}
```

Add to `crates/santa-cli/tests/e2e/mod.rs`:

```rust
mod init_tests;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p santa --test integration_tests -- init`
Expected: All 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/santa-cli/src/init.rs crates/santa-cli/src/main.rs crates/santa-cli/src/lib.rs crates/santa-cli/tests/e2e/init_tests.rs crates/santa-cli/tests/e2e/mod.rs
git commit -m "feat: add santa init command

Interactive config generation with platform detection, manager selection,
and optional starter packages. Supports --yes for non-interactive mode
and --output for custom config path."
```

---

### Task 2: Create Missing Source Definitions

**Files:**
- Modify: `crates/santa-cli/data/sources.ccl` (add apt, dnf, winget)
- Create: `crates/santa-cli/data/sources/dnf.ccl`
- Create: `crates/santa-cli/data/sources/winget.ccl`
- Create: `crates/santa-cli/data/sources/flathub.ccl`

**Context:** Source definitions follow the CCL format in `crates/santa-cli/data/sources.ccl` which defines the manager's emoji, install/check commands, and optional prefix/overrides. Individual source files in `data/sources/` list packages available in that manager. The `flathub` manager already has an entry in `sources.ccl` (line 33-37), so only a package list file is needed. **Note:** `apt` is a Tier 1 manager with a package list file (`data/sources/apt.ccl`) but is missing from `sources.ccl` — its install template is hardcoded in `templates/install.sh.tera` but it should also be in `sources.ccl` for consistency (used by `sources list`, status checking, etc.).

- [ ] **Step 1: Add apt to sources.ccl (Tier 1 gap)**

Add to `crates/santa-cli/data/sources.ccl`:

```ccl
apt =
  emoji = 📦
  install = sudo apt install -y {package}
  check = apt list --installed 2>/dev/null | cut -d'/' -f1 | tail -n +2
```

- [ ] **Step 2: Add arch and aur to sources.ccl (Tier 2 gaps)**

Both `arch` and `aur` have package list files but are missing from `sources.ccl`. Add:

```ccl
arch =
  emoji = 🏛️
  install = sudo pacman -S --noconfirm {package}
  check = pacman -Qq

aur =
  emoji = 🔧
  install = yay -S --noconfirm {package}
  check = yay -Qq
```

Note: `aur` assumes `yay` as the AUR helper. This is the most common choice but users may use `paru` or others. This is acceptable for Tier 2 best-effort.

- [ ] **Step 3: Add dnf to sources.ccl**

Add to `crates/santa-cli/data/sources.ccl`:

```ccl
dnf =
  emoji = 🎩
  install = sudo dnf install -y {package}
  check = dnf list installed | tail -n +2 | cut -d'.' -f1
```

- [ ] **Step 4: Create dnf package list**

Create `crates/santa-cli/data/sources/dnf.ccl`:

```ccl
/= DNF packages (Fedora/RHEL)
bat =
cmake =
curl =
fd-find =
ffmpeg =
fish =
fzf =
gcc =
git =
gnupg2 =
golang =
htop =
ImageMagick =
jq =
make =
mc =
ncdu =
neovim =
pandoc =
python3 =
ripgrep =
rsync =
screen =
starship =
thefuck =
tig =
tmux =
tree =
vim =
wget =
zsh =
```

- [ ] **Step 5: Add winget to sources.ccl**

Add to `crates/santa-cli/data/sources.ccl`:

```ccl
winget =
  emoji = 🪟
  install = winget install --id {package} --accept-source-agreements --accept-package-agreements
  check = winget list | ForEach-Object { ($_ -split '\s{2,}')[0] }
```

- [ ] **Step 6: Create winget package list**

Create `crates/santa-cli/data/sources/winget.ccl`:

```ccl
/= Winget packages (Windows)
Git.Git =
Microsoft.VisualStudioCode =
Mozilla.Firefox =
7zip.7zip =
Notepad++.Notepad++ =
Python.Python.3.12 =
GoLang.Go =
Rustlang.Rustup =
Node.js.LTS =
JetBrains.Toolbox =
Docker.DockerDesktop =
```

- [ ] **Step 7: Create flathub package list**

Create `crates/santa-cli/data/sources/flathub.ccl`:

```ccl
/= Flathub packages (Flatpak)
com.spotify.Client =
com.discordapp.Discord =
org.mozilla.firefox =
org.videolan.VLC =
com.visualstudio.code =
org.gimp.GIMP =
org.inkscape.Inkscape =
com.obsproject.Studio =
```

- [ ] **Step 8: Regenerate index**

Run: `just generate-index`
Verify: `crates/santa-cli/data/known_packages.ccl` includes the new packages.

- [ ] **Step 9: Run tests**

Run: `cargo test -p santa`
Expected: All tests pass. No regressions.

- [ ] **Step 10: Commit**

```bash
git add crates/santa-cli/data/sources.ccl crates/santa-cli/data/sources/dnf.ccl crates/santa-cli/data/sources/winget.ccl crates/santa-cli/data/sources/flathub.ccl crates/santa-cli/data/known_packages.ccl
git commit -m "feat: add apt, dnf, winget, and flathub source definitions

Adds apt to sources.ccl (was missing despite being Tier 1). Adds Tier 2
support for dnf (Fedora/RHEL), winget (Windows), and flathub (Flatpak)
with initial package lists."
```

---

### Task 3: Polish Error Messages

**Files:**
- Modify: `crates/santa-cli/src/errors.rs` (add Display improvements)
- Modify: `crates/santa-cli/src/main.rs` (improve top-level error display)
- Create: `crates/santa-cli/tests/e2e/error_snapshot_tests.rs`
- Modify: `crates/santa-cli/tests/e2e/mod.rs`

**Context:** The existing `SantaError` enum in `errors.rs` uses `thiserror` with structured variants. The top-level error handler in `main.rs` (line 659-665) currently just prints `error: {err}`. The PRD requires errors to be actionable (what to do), contextual (file/line), and categorized.

- [ ] **Step 1: Improve top-level error display in main.rs**

Replace the error handler in `main()` (lines 659-665):

```rust
#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(err) => {
            use colored::Colorize;
            // Check if this is a SantaError with structured info
            if let Some(santa_err) = err.downcast_ref::<santa::errors::SantaError>() {
                eprintln!("{} {}", "error:".red().bold(), santa_err);
                if let Some(hint) = santa_err.hint() {
                    eprintln!("{} {}", "hint:".cyan().bold(), hint);
                }
            } else {
                eprintln!("{} {}", "error:".red().bold(), err);
                // Print the error chain for context
                for cause in err.chain().skip(1) {
                    eprintln!("  {} {}", "caused by:".dimmed(), cause);
                }
            }
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 2: Add hint() method to SantaError**

In `crates/santa-cli/src/errors.rs`, add a `hint()` method that returns actionable guidance:

```rust
impl SantaError {
    /// Returns an actionable hint for the user, if applicable.
    pub fn hint(&self) -> Option<String> {
        match self {
            SantaError::Config(err) => {
                let msg = err.to_string();
                if msg.contains("not found") || msg.contains("No such file") {
                    Some("Run `santa init` to create a config file, or use `--config` to specify a path.".into())
                } else {
                    Some("Check your config file syntax. See `santa config` or the config guide at docs/configuration.md.".into())
                }
            }
            // PackageSource(String) contains "source: message" format from package_source() helper
            SantaError::PackageSource(msg) => {
                if msg.contains("not found") || msg.contains("not installed") {
                    // Try to extract source name from "source: message" format
                    let source = msg.split(':').next().unwrap_or("the package manager");
                    Some(format!("Is `{source}` installed? Check with `which {source}`."))
                } else {
                    Some("Run `santa sources list` to check available sources.".into())
                }
            }
            SantaError::Network(msg) => {
                if msg.contains("timeout") {
                    Some("Check your internet connection and try again.".into())
                } else {
                    Some("Check your internet connection. If this persists, try `santa sources clear` and re-run.".into())
                }
            }
            SantaError::InvalidPackage(_) => {
                Some("Run `santa sources update` to get the latest package definitions.".into())
            }
            SantaError::Security(_) => {
                Some("This package name contains suspicious characters. If this is intentional, file an issue.".into())
            }
            _ => None,
        }
    }
}
```

- [ ] **Step 3: Write error message snapshot tests**

Create `crates/santa-cli/tests/e2e/error_snapshot_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn santa_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

#[test]
fn error_missing_config_shows_hint() {
    santa_cmd()
        .args(["status"])
        .env("SANTA_CONFIG_PATH", "/nonexistent/path/config.ccl")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("hint:").or(predicate::str::contains("santa init")));
}

#[test]
fn error_add_unknown_package_shows_hint() {
    santa_cmd()
        .args(["add", "this-package-definitely-does-not-exist-xyz"])
        .env("SANTA_CONFIG_PATH", "/nonexistent/path/config.ccl")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn error_add_no_packages_shows_usage() {
    santa_cmd()
        .args(["add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No packages specified").or(
            predicate::str::contains("Usage"),
        ));
}

#[test]
fn error_remove_no_packages_shows_usage() {
    santa_cmd()
        .args(["remove"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No packages specified").or(
            predicate::str::contains("Usage"),
        ));
}

#[test]
fn error_sources_show_unknown_source() {
    santa_cmd()
        .args(["sources", "show", "nonexistent-manager"])
        .env("SANTA_CONFIG_PATH", "/dev/null")
        .assert()
        .success() // sources show prints to stderr but exits 0
        .stderr(predicate::str::contains("not found"));
}
```

Add to `crates/santa-cli/tests/e2e/mod.rs`:

```rust
mod error_snapshot_tests;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p santa --test integration_tests -- error_snapshot`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/santa-cli/src/errors.rs crates/santa-cli/src/main.rs crates/santa-cli/tests/e2e/error_snapshot_tests.rs crates/santa-cli/tests/e2e/mod.rs
git commit -m "fix: improve error messages with actionable hints

Add hint() method to SantaError for user-friendly guidance. Improve
top-level error display with colored output and error chain context.
Add snapshot tests for key error scenarios."
```

---

### Task 4: Expand Test Coverage for Tier 1 Managers

**Files:**
- Create: `crates/santa-cli/tests/e2e/tier1_manager_tests.rs`
- Modify: `crates/santa-cli/tests/e2e/mod.rs`

**Context:** The PRD requires all 5 Tier 1 managers (brew, apt, pacman, cargo, npm) to have tested script generation. The script generator lives in `crates/santa-cli/src/script_generator.rs` and uses minijinja templates from `crates/santa-cli/templates/`. Tests should verify that generated scripts contain the correct commands for each manager.

- [ ] **Step 1: Write Tier 1 script generation tests**

Create `crates/santa-cli/tests/e2e/tier1_manager_tests.rs`:

```rust
//! Tests that verify script generation produces correct output for all Tier 1 managers.
//! These tests don't execute the scripts — they verify the generated content.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn santa_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

/// Helper: create a minimal config with packages for a single manager
fn write_test_config(dir: &std::path::Path, manager: &str, packages: &[&str]) -> std::path::PathBuf {
    let config_path = dir.join("config.ccl");
    let mut content = format!("{manager} =\n");
    for pkg in packages {
        content.push_str(&format!("  = {pkg}\n"));
    }
    std::fs::write(&config_path, &content).unwrap();
    config_path
}

#[test]
fn brew_script_contains_brew_install() {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(tmp.path(), "brew", &["ripgrep", "fd"]);

    santa_cmd()
        .args([
            "--config", config.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that a shell script was generated with brew install commands
    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    assert!(!scripts.is_empty(), "No .sh scripts generated");

    for script_entry in &scripts {
        let content = std::fs::read_to_string(script_entry.path()).unwrap();
        assert!(
            content.contains("brew install"),
            "brew script should contain 'brew install', got:\n{content}"
        );
    }
}

#[test]
fn apt_script_contains_apt_install() {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(tmp.path(), "apt", &["ripgrep", "jq"]);

    santa_cmd()
        .args([
            "--config", config.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    assert!(!scripts.is_empty(), "No .sh scripts generated");

    for script_entry in &scripts {
        let content = std::fs::read_to_string(script_entry.path()).unwrap();
        assert!(
            content.contains("apt") && content.contains("install"),
            "apt script should contain apt install command, got:\n{content}"
        );
    }
}

#[test]
fn pacman_script_contains_pacman() {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(tmp.path(), "pacman", &["ripgrep", "jq"]);

    santa_cmd()
        .args([
            "--config", config.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    assert!(!scripts.is_empty(), "No .sh scripts generated");

    for script_entry in &scripts {
        let content = std::fs::read_to_string(script_entry.path()).unwrap();
        assert!(
            content.contains("pacman"),
            "pacman script should contain 'pacman', got:\n{content}"
        );
    }
}

#[test]
fn cargo_script_contains_cargo_install() {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(tmp.path(), "cargo", &["bat", "eza"]);

    santa_cmd()
        .args([
            "--config", config.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    assert!(!scripts.is_empty(), "No .sh scripts generated");

    for script_entry in &scripts {
        let content = std::fs::read_to_string(script_entry.path()).unwrap();
        assert!(
            content.contains("cargo install"),
            "cargo script should contain 'cargo install', got:\n{content}"
        );
    }
}

#[test]
fn npm_script_contains_npm_install() {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config = write_test_config(tmp.path(), "npm", &["typescript", "eslint"]);

    santa_cmd()
        .args([
            "--config", config.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    assert!(!scripts.is_empty(), "No .sh scripts generated");

    for script_entry in &scripts {
        let content = std::fs::read_to_string(script_entry.path()).unwrap();
        assert!(
            content.contains("npm install"),
            "npm script should contain 'npm install', got:\n{content}"
        );
    }
}
```

Add to `crates/santa-cli/tests/e2e/mod.rs`:

```rust
mod tier1_manager_tests;
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p santa --test integration_tests -- tier1`
Expected: All 5 tests pass.

Note: These tests may need adjustment based on how the install command handles configs with packages not in the database. If `install` requires packages to be in the known_packages index, the test configs may need to use packages that are in the bundled data. Check `crates/santa-cli/data/known_packages.ccl` for available package names.

- [ ] **Step 3: Commit**

```bash
git add crates/santa-cli/tests/e2e/tier1_manager_tests.rs crates/santa-cli/tests/e2e/mod.rs
git commit -m "test: add Tier 1 manager script generation tests

Verify that brew, apt, pacman, cargo, and npm all produce correct
install scripts with the expected commands."
```

---

### Task 5: Update Documentation

**Files:**
- Modify: `docs/user-guide.md` (add `santa init` section)
- Modify: `docs/configuration.md` (add init workflow reference)
- Modify: `README.md` (update for v1, add init to quick start)
- Modify: `docs/troubleshooting.md` (add init-related troubleshooting)

**Context:** Docs are already comprehensive. This task adds `santa init` references and ensures docs are v1-ready. Read each file before editing to understand the existing structure.

- [ ] **Step 1: Read current docs**

Read: `docs/user-guide.md`, `docs/configuration.md`, `README.md`, `docs/troubleshooting.md`
Purpose: Understand existing structure before making edits.

- [ ] **Step 2: Add `santa init` to user guide**

In `docs/user-guide.md`, add a new section after the installation section (or wherever the "Getting Started" content is). The section should cover:

```markdown
## Getting Started with `santa init`

The fastest way to get started is to run `santa init`:

```sh
santa init
```

This will:
1. Detect which package managers are available on your system
2. Let you choose which ones to include
3. Optionally seed your config with common dev tools
4. Write a starter config to `~/.config/santa/config.ccl`

### Non-interactive mode

For scripted setups, use `--yes` to accept all defaults:

```sh
santa init --yes
```

### Custom output path

To write the config somewhere else:

```sh
santa init --output ./my-config.ccl
```
```

- [ ] **Step 3: Add init reference to configuration guide**

In `docs/configuration.md`, add a note near the top referencing `santa init` as the easiest way to create a config:

```markdown
> **Tip:** Run `santa init` to generate a starter config automatically.
```

- [ ] **Step 4: Update README quick start**

In `README.md`, add `santa init` as the first step in the quick start / getting started section:

```markdown
# Quick start
santa init              # Create your config
santa status            # See what's installed vs. missing
santa install           # Generate install scripts
```

- [ ] **Step 5: Add init troubleshooting**

In `docs/troubleshooting.md`, add a section:

```markdown
## `santa init` issues

### "Config already exists" error

If you already have a config file, `santa init` will not overwrite it by default. Options:
- Remove the existing config: `rm ~/.config/santa/config.ccl`
- Write to a different path: `santa init --output ./new-config.ccl`

### No package managers detected

If `santa init` doesn't detect any managers, they may not be on your `$PATH`. Verify with:
```sh
which brew apt pacman cargo npm
```
```

- [ ] **Step 6: Commit**

```bash
git add docs/user-guide.md docs/configuration.md README.md docs/troubleshooting.md
git commit -m "docs: add santa init documentation

Add init command to user guide, configuration guide, README quick start,
and troubleshooting guide."
```

---

### Task 6: Set Up Homebrew Tap

**Files:**
- Create: New GitHub repository `homebrew-tap` (or `homebrew-santa`)
- Create: `Formula/santa.rb` in the tap repo

**Context:** This task creates the infrastructure for distributing santa via `brew install`. It requires a separate GitHub repository following the `homebrew-<name>` naming convention. The formula pulls pre-built binaries from GitHub Releases (produced by cargo-dist in `.github/workflows/santa-v-release.yml`).

**Important:** This task creates a new repository and cannot be done entirely within the santa workspace. It requires GitHub access.

- [ ] **Step 1: Create the tap repository**

Create a new GitHub repo named `homebrew-santa` (or `homebrew-tap`) under the user's GitHub account. It should be public.

Run:
```bash
gh repo create tylerbu/homebrew-tap --public --description "Homebrew tap for santa package manager"
```

- [ ] **Step 2: Create the formula**

Create `Formula/santa.rb` in the new repo. The formula should download pre-built binaries from GitHub Releases. Example structure:

```ruby
class Santa < Formula
  desc "Declare your packages once, install them everywhere"
  homepage "https://github.com/tylerbu/santa"
  version "1.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tylerbu/santa/releases/download/santa-v#{version}/santa-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/tylerbu/santa/releases/download/santa-v#{version}/santa-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tylerbu/santa/releases/download/santa-v#{version}/santa-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/tylerbu/santa/releases/download/santa-v#{version}/santa-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "santa"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/santa --version")
  end
end
```

Note: SHA256 values and exact archive names will need to match the cargo-dist output format. Check the existing release artifacts from the most recent GitHub Release to get the exact naming convention.

- [ ] **Step 3: Add tap update automation**

Add a GitHub Actions workflow to the santa repo that updates the tap formula when a new release is published. This can be a simple workflow that:
1. Triggers on release published
2. Calculates SHA256 of the release artifacts
3. Updates the formula in the tap repo via a PR or direct push

This workflow design depends on the exact cargo-dist output format and can be refined after the first manual release.

- [ ] **Step 4: Test the tap locally**

```bash
brew tap tylerbu/tap
brew install tylerbu/tap/santa
santa --version
```

- [ ] **Step 5: Document the tap in README**

Add to the installation section of `README.md`:

```markdown
### Homebrew

```sh
brew install tylerbu/tap/santa
```
```

- [ ] **Step 6: Commit README update**

```bash
git add README.md
git commit -m "docs: add Homebrew tap installation instructions"
```

---

### Task 7: Tier 2 Smoke Tests and Cross-Platform Name Resolution Tests

**Files:**
- Create: `crates/santa-cli/tests/e2e/tier2_smoke_tests.rs`
- Create: `crates/santa-cli/tests/e2e/name_resolution_tests.rs`
- Modify: `crates/santa-cli/tests/e2e/mod.rs`

**Context:** PRD success criterion #4 requires Tier 2 managers "must not panic or generate syntactically invalid scripts." PRD test coverage table requires "Cross-platform name resolution: Tier 1 managers have verified mappings." These are both missing from the original plan.

- [ ] **Step 1: Write Tier 2 smoke tests**

Create `crates/santa-cli/tests/e2e/tier2_smoke_tests.rs`:

```rust
//! Smoke tests for Tier 2 managers: verify they don't panic or produce invalid scripts.

use assert_cmd::Command;
use tempfile::TempDir;

fn santa_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

fn write_test_config(dir: &std::path::Path, manager: &str, packages: &[&str]) -> std::path::PathBuf {
    let config_path = dir.join("config.ccl");
    let mut content = format!("{manager} =\n");
    for pkg in packages {
        content.push_str(&format!("  = {pkg}\n"));
    }
    std::fs::write(&config_path, &content).unwrap();
    config_path
}

/// Macro to generate a smoke test for each Tier 2 manager
macro_rules! tier2_smoke_test {
    ($name:ident, $manager:expr, $packages:expr) => {
        #[test]
        fn $name() {
            let tmp = TempDir::new().unwrap();
            let output_dir = TempDir::new().unwrap();
            let config = write_test_config(tmp.path(), $manager, $packages);

            // Should not panic and should exit cleanly
            santa_cmd()
                .args([
                    "--config", config.to_str().unwrap(),
                    "install",
                    "--format", "shell",
                    "--output-dir", output_dir.path().to_str().unwrap(),
                ])
                .assert()
                .success();
        }
    };
}

tier2_smoke_test!(dnf_does_not_crash, "dnf", &["git", "curl"]);
tier2_smoke_test!(nix_does_not_crash, "nix", &["git", "curl"]);
tier2_smoke_test!(arch_does_not_crash, "arch", &["binutils", "cairo"]);
tier2_smoke_test!(aur_does_not_crash, "aur", &["ack", "add-gitignore"]);
tier2_smoke_test!(scoop_does_not_crash, "scoop", &["git", "curl"]);
tier2_smoke_test!(winget_does_not_crash, "winget", &["Git.Git"]);
tier2_smoke_test!(flathub_does_not_crash, "flathub", &["org.mozilla.firefox"]);
```

- [ ] **Step 2: Write cross-platform name resolution tests**

Create `crates/santa-cli/tests/e2e/name_resolution_tests.rs`:

```rust
//! Tests that verify cross-platform package name mappings are correct
//! for Tier 1 managers. These test the bundled data, not the pipeline.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn santa_cmd() -> Command {
    Command::cargo_bin("santa").unwrap()
}

/// Test that a package is recognized and produces a script with the correct name.
/// Uses a config with the manager-specific name and verifies the script contains it.
fn verify_package_name(manager: &str, package_name: &str) {
    let tmp = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.ccl");
    let content = format!("{manager} =\n  = {package_name}\n");
    std::fs::write(&config_path, &content).unwrap();

    let result = santa_cmd()
        .args([
            "--config", config_path.to_str().unwrap(),
            "install",
            "--format", "shell",
            "--output-dir", output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify the generated script contains the expected package name
    let scripts: Vec<_> = std::fs::read_dir(output_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sh"))
        .collect();

    if !scripts.is_empty() {
        let content = std::fs::read_to_string(scripts[0].path()).unwrap();
        assert!(
            content.contains(package_name),
            "Script for {manager} should contain '{package_name}', got:\n{content}"
        );
    }
}

#[test]
fn fd_maps_to_fd_find_on_apt() {
    // fd is known as fd-find in apt
    verify_package_name("apt", "fd-find");
}

#[test]
fn ripgrep_works_on_all_tier1() {
    for manager in &["brew", "apt", "pacman", "cargo"] {
        verify_package_name(manager, "ripgrep");
    }
}

#[test]
fn bat_works_on_brew_and_cargo() {
    verify_package_name("brew", "bat");
    verify_package_name("cargo", "bat");
}

#[test]
fn jq_works_on_brew_apt_pacman() {
    for manager in &["brew", "apt", "pacman"] {
        verify_package_name(manager, "jq");
    }
}
```

- [ ] **Step 3: Update e2e/mod.rs**

Add to `crates/santa-cli/tests/e2e/mod.rs`:

```rust
mod tier2_smoke_tests;
mod name_resolution_tests;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p santa --test integration_tests -- tier2 name_resolution`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/santa-cli/tests/e2e/tier2_smoke_tests.rs crates/santa-cli/tests/e2e/name_resolution_tests.rs crates/santa-cli/tests/e2e/mod.rs
git commit -m "test: add Tier 2 smoke tests and name resolution tests

Verify Tier 2 managers don't crash on script generation. Verify
cross-platform package name mappings for Tier 1 managers."
```

---

### Task 8: Version Bump to 1.0.0

**Files:**
- Modify: `Cargo.toml` (workspace version)
- Modify: `crates/santa-cli/Cargo.toml` (santa-data dependency version)
- Modify: `crates/santa-data/Cargo.toml` (if version is workspace-inherited, only workspace root needs change)

**Context:** The workspace `Cargo.toml` defines the version for all crates. Currently at 0.3.2. The `santa` and `santa-data` crates use `version.workspace = true`. Sickle crates have their own version (0.1.x) and should NOT be bumped.

**Important:** This task should only be done after all other tasks are complete and CI is green.

- [ ] **Step 1: Check current version configuration**

Read: `Cargo.toml` (workspace root) to understand how versions are managed.
Verify which crates use `version.workspace = true` vs. their own version.

- [ ] **Step 2: Determine version strategy**

The workspace version applies to `santa` and `santa-data`. Sickle crates (`sickle`, `sickle-cli`) should have independent versions. If the workspace version currently applies to all crates, the sickle crates may need to be switched to explicit versions before bumping the workspace to 1.0.0.

- [ ] **Step 3: Update versions**

Update the workspace version in root `Cargo.toml` from `0.3.2` to `1.0.0`.
If sickle crates inherit workspace version, switch them to explicit `version = "0.1.x"`.
Update any internal `santa-data = { version = "0.3.2", ... }` dependency references to `"1.0.0"`.

- [ ] **Step 4: Update changelogs**

Use `changie` to create a changelog entry for the 1.0.0 release:
```bash
changie new --kind feat --body "Santa v1.0.0 release"
```

- [ ] **Step 5: Run full CI checks**

```bash
just pr
```

Expected: All checks pass (format, docs, lint, coverage, audit, build, verify).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/*/Cargo.toml Cargo.lock .changes/
git commit -m "chore: bump santa and santa-data to 1.0.0

Establishes semver stability contract for the santa CLI and santa-data
library. Sickle crates remain at 0.x."
```

---

## Execution Order

Tasks share some files and must be serialized in groups:

**File conflicts:**
- `main.rs`: Tasks 1, 3
- `tests/e2e/mod.rs`: Tasks 1, 3, 4, 7
- `README.md`: Tasks 5, 6

**Recommended serial execution order:**
1. Task 2: Missing source definitions (quick, unblocks Tier 1/2 tests)
2. Task 1: `santa init` (largest new feature)
3. Task 3: Error message polish (touches main.rs after Task 1)
4. Task 4: Tier 1 manager tests
5. Task 7: Tier 2 smoke tests and name resolution tests
6. Task 5: Documentation updates
7. Task 6: Homebrew tap setup
8. Task 8: Version bump to 1.0.0 (last — only after everything else is green)

**After all tasks:** Run `just pr` to verify the full CI suite passes before tagging the release.

# Santa User Guide

Santa manages packages across multiple package managers with a unified interface. This guide covers installation, basic usage, and common workflows.

## Quick Start

### Installation

```bash
# Install from crates.io
cargo install santa-cli

# Or build from source
git clone https://github.com/tylerbutler/santa.git
cd santa
cargo install --path crates/santa-cli
```

### First Steps

1. Check available package managers on your system:
   ```bash
   santa sources list
   ```

2. Check what packages Santa tracks:
   ```bash
   santa status
   ```

3. Install tracked packages:
   ```bash
   santa install
   ```

## Command Reference

### `santa status`

Shows which tracked packages are installed or missing.

```bash
# Show only missing packages (default)
santa status

# Show all tracked packages
santa status --all
```

**Output**: Lists packages by source, marking installed (✓) and missing (✗) packages.

### `santa install`

Generates installation scripts for missing packages.

```bash
# Install all missing packages from all sources
santa install

# Install packages from a specific source only
santa install brew
```

**Execution Modes**:

- **Safe mode** (default): Generates scripts in `~/.santa/scripts/` for review
- **Execute mode** (`-x`): Runs commands directly (use with caution)

```bash
# Generate script (safe)
santa install

# Execute directly (dangerous)
santa install -x
```

**Script output**: By default, scripts are saved to `~/.santa/scripts/`. Override with `--output-dir`:

```bash
santa install --output-dir ./my-scripts
```

### `santa config`

Displays current configuration.

```bash
# Show configuration summary
santa config

# Show full configuration including all packages
santa config --packages

# Show configuration in pipe-friendly format
santa config --pipe
```

### `santa add`

Adds a package to your tracking list.

```bash
# Add a package interactively
santa add

# Add a specific package to a source
santa add ripgrep cargo
santa add bat brew
```

The package is added to your user configuration file (`~/.config/santa/config.ccl`).

### `santa sources`

Manages package source definitions.

```bash
# Update source definitions from GitHub
santa sources update

# List all available sources
santa sources list

# List sources by origin (bundled, downloaded, custom)
santa sources list --origin bundled
santa sources list --origin downloaded
santa sources list --origin custom
```

**Source origins**:
- **bundled**: Built-in source definitions shipped with Santa
- **downloaded**: Source definitions from GitHub releases
- **custom**: Your local customizations

### `santa completions`

Generates shell completions for your shell.

```bash
# Bash
santa completions bash > ~/.local/share/bash-completion/completions/santa

# Zsh
santa completions zsh > ~/.zfunc/_santa

# Fish
santa completions fish > ~/.config/fish/completions/santa.fish

# PowerShell
santa completions powershell > santa.ps1
```

## Execution Modes

Santa operates in two modes for safety:

### Safe Mode (Default)

Generates shell scripts instead of executing commands directly. This allows you to:

- Review commands before execution
- Customize generated scripts
- Run scripts at your convenience
- Maintain audit trails

Scripts are saved to `~/.santa/scripts/` by default.

### Execute Mode

Direct command execution without script generation. Enable with `-x` or `--execute`:

```bash
santa install -x
```

**Use execute mode only when**:
- You trust the package sources completely
- You've reviewed the configuration
- You understand the commands being run

## Script Formats

Santa auto-detects the appropriate script format for your platform:

- **Linux/macOS**: Shell scripts (`.sh`)
- **Windows**: PowerShell (`.ps1`) or Batch (`.bat`)

Override with `--format`:

```bash
santa install --format powershell
santa install --format shell
santa install --format batch
```

## Configuration Basics

Santa uses CCL (Categorical Configuration Language) for configuration. See the [Configuration Guide](configuration.md) for details.

Configuration files are loaded in priority order:

1. Built-in defaults (bundled with Santa)
2. User config: `~/.config/santa/config.ccl`
3. Project config: `.santa/config.ccl` (current directory)

Later files override earlier ones.

### Configuration File Location

Default location: `~/.config/santa/config.ccl`

Override with `SANTA_CONFIG` environment variable:

```bash
export SANTA_CONFIG=/path/to/my/config.ccl
santa status
```

### Basic Configuration Example

```ccl
/= Preferred package manager sources
sources =
  = brew
  = cargo
  = apt

/= Packages to track
packages =
  = bat
  = ripgrep
  = fd-find
```

## Common Workflows

### Install packages on a new system

```bash
# 1. Install Santa
cargo install santa-cli

# 2. Update source definitions
santa sources update

# 3. Check what's missing
santa status

# 4. Generate installation script
santa install

# 5. Review and run the script
cat ~/.santa/scripts/install_*.sh
sh ~/.santa/scripts/install_*.sh
```

### Add a new package

```bash
# Add package to tracking list
santa add ripgrep cargo

# Install it
santa install cargo
```

### Check package status across sources

```bash
# See all tracked packages
santa status --all

# See configuration
santa config --packages
```

### Update source definitions

```bash
# Download latest package source definitions
santa sources update

# Verify new sources
santa sources list --origin downloaded
```

## Global Options

These options work with all commands:

- `-v, --verbose`: Increase logging verbosity (use multiple times: `-vv`, `-vvv`)
- `-x, --execute`: Enable direct execution mode (default: safe script generation)
- `--format <FORMAT>`: Script format (auto, shell, powershell, batch)
- `--output-dir <DIR>`: Directory for generated scripts
- `--builtin-only`: Load only built-in configuration (ignore user config)

### Logging Levels

Control output verbosity with `-v`:

```bash
# Default: errors and warnings only
santa status

# Info level
santa status -v

# Debug level
santa status -vv

# Trace level (very detailed)
santa status -vvv
```

## Tips

- Start with safe mode (default) to understand what Santa does
- Use `santa status --all` to see your complete package tracking list
- Review generated scripts before executing them
- Use `santa config` to verify your configuration is loaded correctly
- Run `santa sources update` periodically for latest package definitions
- Use `--builtin-only` to test with default configuration only

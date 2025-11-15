# Santa User Guide

**Complete guide to using Santa, the modern package manager meta-tool**

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Configuration](#configuration)
5. [Commands](#commands)
6. [Advanced Usage](#advanced-usage)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

---

## Introduction

Santa is a package manager meta-tool that simplifies managing packages across different platforms and package managers. Whether you're on Linux, macOS, or Windows, Santa provides a unified interface to install and manage your essential development tools.

### Why Santa?

- **Cross-platform**: Works seamlessly on Linux, macOS, and Windows
- **Unified interface**: One command for multiple package managers
- **Safe by default**: Generates scripts for review before execution
- **Performance**: 67-90% faster through concurrent operations
- **Flexible**: CCL configuration with hot-reloading support

### Who Should Use Santa?

Santa is perfect for developers who:
- Work across multiple operating systems or architectures
- Want a consistent toolkit across all machines
- Prefer to review package installation scripts before execution
- Need fast, concurrent package operations

---

## Installation

### From Source (Recommended)

Requires Rust 1.80.0 or later:

```bash
# Install from GitHub
cargo install --git https://github.com/tylerbutler/santa

# Or clone and build locally
git clone https://github.com/tylerbutler/santa.git
cd santa
cargo install --path .
```

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/tylerbutler/santa/releases):

```bash
# Linux (x64)
curl -L https://github.com/tylerbutler/santa/releases/latest/download/santa-linux-x64.tar.gz | tar xz
sudo mv santa /usr/local/bin/

# macOS
# Download and install from releases page

# Windows
# Download .exe from releases page
```

### Verify Installation

```bash
santa --version
```

---

## Quick Start

### 1. Check Package Status

See which packages are available or missing:

```bash
# Show only missing packages
santa status

# Show all packages
santa status --all
```

### 2. Generate Installation Scripts

Santa generates safe scripts by default:

```bash
# Generate scripts for all missing packages
santa install

# Scripts are saved to current directory by default
# Review and execute manually:
./install-<source>.sh
```

### 3. View Configuration

```bash
# Show current configuration
santa config

# Show package definitions
santa config --packages

# Show source definitions
santa config --builtin
```

### 4. Shell Completions

```bash
# Bash
santa completions bash >> ~/.bashrc

# Zsh
santa completions zsh >> ~/.zshrc

# Fish
santa completions fish >> ~/.config/fish/config.fish
```

---

## Configuration

### Configuration File Location

Santa looks for configuration in the following locations (in order):

1. `$SANTA_CONFIG_PATH` (if set)
2. `~/.config/santa/config.yaml`
3. Built-in defaults

### CCL Configuration Format

Santa uses CCL (Categorical Configuration Language) for modern, expressive configuration:

```ccl
# santa-config.ccl

# Package sources in order of preference
sources = [brew, cargo, apt, pacman, scoop, nix]

# Packages to manage
packages = {
  # Command-line tools
  bat: "Better cat with syntax highlighting"
  ripgrep: "Fast grep alternative"
  fd: "Better find"
  exa: "Modern ls replacement"

  # Development tools
  git: "Version control"
  rust: "Rust toolchain"
  node: "Node.js runtime"
  python: "Python interpreter"
}

# Source-specific package names
package_names = {
  brew: {
    rust: "rustup"
    node: "node@20"
  }
  apt: {
    fd: "fd-find"
  }
}
```

### Legacy YAML Support

Santa still supports YAML configuration with automatic migration:

```yaml
# ~/.config/santa/config.yaml
sources:
  - brew
  - cargo
  - apt

packages:
  - bat
  - ripgrep
  - fd
  - git
```

### Environment Variables

Override configuration with environment variables:

```bash
# Core settings
export SANTA_LOG_LEVEL="debug"           # Logging: trace, debug, info, warn, error
export SANTA_CONFIG_PATH="/path/config"  # Custom config location
export SANTA_BUILTIN_ONLY="true"         # Use only built-in config

# Sources and packages
export SANTA_SOURCES="brew,cargo,apt"    # Override sources (comma-separated)
export SANTA_PACKAGES="git,rust,node"    # Override packages (comma-separated)

# Performance
export SANTA_CACHE_TTL_SECONDS="600"     # Cache TTL (default: 300)
export SANTA_CACHE_SIZE="2000"           # Max cache entries (default: 1000)

# Features
export SANTA_HOT_RELOAD="true"           # Enable config hot-reloading
export SANTA_VERBOSE="2"                 # Verbosity level (0-3)
```

---

## Commands

### `santa status`

Display package availability across all enabled sources.

```bash
# Show only missing packages
santa status

# Show all packages
santa status --all

# With verbose logging
santa status -vv
```

**Output:**
```
brew (5 packages total)
Package    Status
ripgrep    ✓ installed
bat        ✗ missing
fd         ✓ installed
```

### `santa install`

Generate installation scripts for missing packages.

```bash
# Generate scripts for all missing packages (safe mode)
santa install

# Generate PowerShell scripts on Windows
santa install --format powershell

# Specify output directory
santa install --output-dir ./scripts

# Direct execution (requires --execute flag and confirmation)
santa install --execute
```

**Safety Note:** By default, Santa generates scripts you can review. Use `--execute` only when you trust the operations.

### `santa config`

Display current configuration.

```bash
# Show user configuration
santa config

# Show package definitions
santa config --packages

# Show source definitions (built-in)
santa config --builtin

# Pipe mode for scripting
santa config --pipe
```

### `santa add`

Add packages to tracking configuration.

```bash
# Interactive mode
santa add

# Specify package and source
santa add neovim brew
```

### `santa completions`

Generate shell completions.

```bash
# Generate for specific shell
santa completions bash
santa completions zsh
santa completions fish
santa completions powershell
```

### Global Flags

Available on all commands:

```bash
-v, --verbose         # Increase logging level (can be used multiple times: -vvv)
-b, --builtin-only   # Use only built-in configuration
-x, --execute        # Enable direct execution (dangerous)
--format <FORMAT>    # Script format: shell, powershell, batch
--output-dir <DIR>   # Output directory for generated scripts
```

---

## Advanced Usage

### Custom Package Sources

Define custom package managers:

```ccl
# Custom source definition
sources = [
  {
    name: "custom-apt"
    manager: "apt-get"
    install_command: "install -y"
    list_command: "list --installed"
    available_check: "dpkg -l"
  }
]
```

### Script Templates

Customize script generation with Tera templates:

1. Create custom templates in `~/.config/santa/templates/`
2. Use template syntax:

```tera
#!/bin/bash
# Generated by Santa v{{ version }}
# Timestamp: {{ timestamp }}

{% for package in packages %}
{{ manager }} install {{ package | shell_escape }}
{% endfor %}
```

### Configuration Hot-Reloading

Enable live configuration updates:

```bash
# Enable hot-reload
export SANTA_HOT_RELOAD=true
santa status

# In another terminal, edit config
vim ~/.config/santa/config.ccl

# Changes are automatically detected and applied
```

### Performance Tuning

```bash
# Increase cache size for large package lists
export SANTA_CACHE_SIZE=5000

# Extend cache TTL to reduce network calls
export SANTA_CACHE_TTL_SECONDS=1800

# Adjust concurrency (default: auto-detected)
export SANTA_MAX_CONCURRENT=20
```

### Integration with Dotfiles

Add Santa to your dotfile management:

```bash
# In your dotfiles setup script
santa install --builtin-only
./install-brew.sh
./install-cargo.sh

# Or with direct execution (if trusted)
santa install --execute --builtin-only
```

---

## Troubleshooting

### Common Issues

#### "Package manager not found"

**Cause:** The package manager (brew, apt, etc.) is not installed.

**Solution:**
```bash
# Install the required package manager first
# For Homebrew on macOS/Linux:
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

#### "Configuration file not found"

**Cause:** No config file exists and built-in config is not enabled.

**Solution:**
```bash
# Use built-in config
santa status --builtin-only

# Or create a config file
mkdir -p ~/.config/santa
cat > ~/.config/santa/config.ccl << 'EOF'
sources = [brew, cargo]
packages = [git, rust]
EOF
```

#### "Permission denied" errors

**Cause:** Package manager requires sudo/admin privileges.

**Solution:**
```bash
# Review generated scripts and run with appropriate privileges
santa install
sudo ./install-apt.sh
```

#### Cache issues

**Cause:** Stale cache causing incorrect status.

**Solution:**
```bash
# Clear cache by restarting Santa
# Or adjust cache TTL
export SANTA_CACHE_TTL_SECONDS=60
santa status
```

### Debug Mode

Enable detailed logging:

```bash
# Trace level (most detailed)
SANTA_LOG_LEVEL=trace santa status

# Or use verbose flags
santa status -vvv
```

### Getting Help

```bash
# Command-specific help
santa install --help
santa status --help

# General help
santa --help
```

---

## FAQ

### How does Santa differ from other package managers?

Santa is a **meta-tool** that orchestrates multiple package managers, not a replacement. It provides a unified interface while using existing package managers (brew, apt, cargo, etc.) under the hood.

### Is it safe to use?

Yes! Santa is **safe by default**:
- Generates scripts for review (not direct execution)
- All inputs are sanitized and escaped
- Uses template-based generation to prevent injection
- Open source and auditable

### Can I use Santa in CI/CD?

Absolutely! Santa is designed for automation:

```bash
# In CI pipeline
santa install --builtin-only
./install-brew.sh
```

### What package managers are supported?

Currently supported:
- **Homebrew** (macOS, Linux)
- **Cargo** (Rust packages)
- **APT** (Debian, Ubuntu)
- **Pacman** (Arch Linux)
- **Scoop** (Windows)
- **Nix** (Universal)

### Can I add custom package managers?

Yes! Define custom sources in your configuration file with the appropriate commands.

### How do I migrate from YAML to CCL?

Santa automatically migrates YAML configs to CCL. Your existing YAML files continue to work, or you can manually convert to CCL format for better features.

### Does Santa require internet access?

Only for checking package availability. Generated scripts work offline once created.

### How do I contribute?

See the [Contributing Guide](../CONTRIBUTING.md) and [Development Setup](../README.md#development).

---

## Further Reading

- [API Documentation](./API.md) - Library usage and programmatic access
- [Architecture Guide](./ARCHITECTURE.md) - Internal design and architecture
- [Contributing Guide](../CONTRIBUTING.md) - Development and contribution guidelines
- [README](../README.md) - Project overview and quick start

---

**Questions or feedback?** [Open an issue](https://github.com/tylerbutler/santa/issues) on GitHub!

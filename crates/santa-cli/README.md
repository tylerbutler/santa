# Santa 🎅

**A modern, high-performance package manager meta-tool that works across platforms.**

Santa helps you install and manage packages across multiple platforms and package managers with a single command. Whether you're switching between macOS, Linux, or Windows, Santa ensures your essential tools are always available.

[![Build Status](https://github.com/tylerbutler/santa/workflows/CI/badge.svg)](https://github.com/tylerbutler/santa/actions)
[![codecov](https://codecov.io/gh/tylerbutler/santa/branch/main/graph/badge.svg)](https://codecov.io/gh/tylerbutler/santa)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Why Santa?

### You regularly use tools not installed by default

You're a modern developer. You prefer `ripgrep` over `grep`, `bat` over `cat`, and `fd` over `find`. The problem? They're not installed by default. You're stuck installing them manually with whatever package manager is available.

Santa gives you **one command** to install your entire "standard developer toolkit."

### You work across different operating systems

Isn't it annoying when you log into a new machine and it doesn't have your preferred tools? Or when your tool isn't installable using `apt`, but you don't remember which package manager _does_ have it?

Santa simplifies this workflow. Santa knows where your packages can be installed from and will install them from the best available source.

## Features

### 🚀 Performance
- **67-90% faster** concurrent package operations via async subprocess execution
- **Professional-grade caching** with TTL and LRU eviction
- **Memory efficient** with zero unnecessary allocations

### 🔧 Cross-Platform
- **Linux, macOS, and Windows** support
- **Multiple package managers**: Homebrew, Cargo, APT, Pacman, AUR, Scoop, Nix
- **Automatic fallback** to available package managers

### 🌟 User-Friendly
- **Configuration hot-reloading** - changes take effect immediately
- **Shell completions** for bash, zsh, and fish
- **Environment variable overrides** for all settings
- **CCL configuration format** - simple and readable

### 🛡️ Reliable
- **Comprehensive error handling** with helpful messages
- **Security hardening** with input sanitization
- **144 comprehensive tests** with >90% code coverage

## Installation

### From Releases (Recommended)

Download pre-built binaries from the [releases page](https://github.com/tylerbutler/santa/releases):

```bash
# Linux (x64)
curl -L https://github.com/tylerbutler/santa/releases/latest/download/santa-linux-x64.tar.gz | tar xz

# macOS (Apple Silicon)
curl -L https://github.com/tylerbutler/santa/releases/latest/download/santa-macos-aarch64.tar.gz | tar xz

# Windows (PowerShell)
irm https://github.com/tylerbutler/santa/releases/latest/download/santa-windows-x64.zip -OutFile santa.zip
Expand-Archive santa.zip
```

### From Source

```bash
cargo install santa-cli
```

### Verify Installation

```bash
santa --version
```

## Quick Start

### 1. Check Status

See what packages are available and their installation status:

```bash
santa status
```

### 2. Install Packages

Install all configured packages:

```bash
santa install
```

Install specific packages:

```bash
santa install ripgrep bat fd
```

### 3. View Configuration

```bash
santa config
```

### 4. Set Up Shell Completions

```bash
# Bash
santa completions bash >> ~/.bashrc

# Zsh
santa completions zsh >> ~/.zshrc

# Fish
santa completions fish > ~/.config/fish/completions/santa.fish
```

## Configuration

Santa uses a CCL (Categorical Configuration Language) configuration file.

**Default location:** `~/.config/santa/config.ccl`

### Basic Configuration

```ccl
/= Package sources in order of preference
sources =
  = brew
  = cargo
  = apt
  = pacman
  = scoop
  = nix

/= Packages to install
packages =
  = bat
  = ripgrep
  = fd
  = exa
  = bottom
  = git-delta
  = chezmoi
```

### Supported Package Managers

| Package Manager | Platforms | Status |
|----------------|-----------|--------|
| **Homebrew** | macOS, Linux | ✅ Full Support |
| **Cargo** | All | ✅ Full Support |
| **APT** | Debian, Ubuntu | ✅ Full Support |
| **Pacman** | Arch Linux | ✅ Full Support |
| **AUR** | Arch Linux | ✅ Full Support |
| **Scoop** | Windows | ✅ Full Support |
| **Nix** | All | ✅ Full Support |

### Environment Variables

Override any configuration setting with environment variables:

```bash
export SANTA_SOURCES="brew,cargo,apt"
export SANTA_PACKAGES="git,rust,ripgrep"
export SANTA_LOG_LEVEL="debug"
export SANTA_BUILTIN_ONLY="true"
```

**Available Variables:**

- `SANTA_LOG_LEVEL` - Set log level (trace, debug, info, warn, error)
- `SANTA_CONFIG_PATH` - Override configuration file path
- `SANTA_SOURCES` - Override package sources (comma-separated)
- `SANTA_PACKAGES` - Override package list (comma-separated)
- `SANTA_BUILTIN_ONLY` - Use builtin configuration only (true/false)
- `SANTA_CACHE_TTL_SECONDS` - Set package cache TTL
- `SANTA_CACHE_SIZE` - Set maximum cache entries
- `SANTA_VERBOSE` - Set verbose logging level (0-3)
- `SANTA_HOT_RELOAD` - Enable configuration hot-reloading (true/false)

## Commands

### `santa status`

Show all packages and their installation status across available package managers.

```bash
santa status

# Show only installed packages
santa status --installed

# Show only missing packages
santa status --missing
```

### `santa install`

Install packages from the configured sources.

```bash
# Install all configured packages
santa install

# Install specific packages
santa install ripgrep bat fd

# Dry run (show what would be installed)
santa install --dry-run
```

### `santa config`

Display current configuration including sources, packages, and settings.

```bash
santa config

# Show configuration file path
santa config --path

# Show resolved configuration (after environment variables)
santa config --resolved
```

### `santa completions`

Generate shell completion scripts.

```bash
# Generate for bash
santa completions bash

# Generate for zsh
santa completions zsh

# Generate for fish
santa completions fish
```

## Advanced Usage

### Custom Configuration File

```bash
santa --config ~/my-config.ccl status
```

### Verbose Output

```bash
# Level 1: Basic info
santa -v install

# Level 2: Detailed info
santa -vv install

# Level 3: Debug info
santa -vvv install
```

### Using with CI/CD

```bash
# Set packages via environment variable
export SANTA_PACKAGES="jq,yq,gh"
santa install

# Use builtin configuration only (no config file)
export SANTA_BUILTIN_ONLY=true
santa install
```

## Troubleshooting

### Package Not Found

If a package isn't found, check:
1. The package name is correct for your package manager
2. Your package manager is in the `sources` list
3. Your package manager is installed and in your PATH

```bash
# Check which package managers are available
santa config
```

### Permission Errors

Some package managers require elevated privileges:

```bash
# Linux/macOS
sudo santa install

# Windows (run PowerShell as Administrator)
santa install
```

### Configuration Not Loading

Verify your configuration file path:

```bash
santa config --path
```

Or specify the path explicitly:

```bash
santa --config ~/.config/santa/config.ccl status
```

## Performance

Santa is designed for speed:

- **67-90% faster** than sequential package operations
- **Parallel execution** of package manager queries
- **Intelligent caching** to avoid redundant operations
- **Async I/O** for non-blocking operations

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/tylerbutler/santa) for development guidelines.

## License

Licensed under the [MIT License](LICENSE).

---

**Made with ❤️ for developers who work across platforms**

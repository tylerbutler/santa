# Santa Configuration Guide

Santa uses CCL (Categorical Configuration Language) for configuration files. This guide explains the format, file locations, and configuration options.

## CCL Format Overview

CCL is a simple, human-readable configuration format. Key features:

- **Hierarchical structure**: Uses indentation and `=` for nesting
- **Comments**: Start with `/=`
- **Lists**: Created with repeated `=` assignments
- **No quotes needed**: Values are plain text

Full CCL documentation: https://ccl.tylerbutler.com

### Basic Syntax

```ccl
/= This is a comment

/= Simple key-value
key = value

/= List of values
items =
  = first
  = second
  = third

/= Nested structure
section =
  subsection =
    key = value
```

## Configuration File Locations

Santa loads configuration in priority order (later files override earlier ones):

1. **Built-in defaults**: Bundled with Santa binary
2. **User configuration**: `~/.config/santa/config.ccl`
3. **Project configuration**: `.santa/config.ccl` (current directory)

### Override with Environment Variable

```bash
export SANTA_CONFIG=/path/to/config.ccl
santa status
```

### Load Only Built-in Configuration

```bash
santa --builtin-only status
```

## Configuration Structure

A complete Santa configuration has two main sections:

### `sources`

Preferred package managers in priority order. Santa uses the first available source for each package.

```ccl
sources =
  = brew
  = cargo
  = apt
  = pacman
```

**Available sources**: `brew`, `cargo`, `apt`, `pacman`, `aur`, `scoop`, `nix`

Santa automatically filters sources to those available on your platform.

### `packages`

Packages to track and install. Format varies based on package naming across sources.

## Package Definition Formats

### Simple Format

For packages with the same name across all sources:

```ccl
packages =
  = bat
  = ripgrep
  = fd
  = jq
```

This expands to check all configured sources for these package names.

### Source-Specific Format

When package names differ across sources or you want to limit which sources provide a package:

```ccl
bat =
  = brew
  = scoop
  = pacman
  = nix

ripgrep =
  = brew
  = cargo
  = apt

fd-find =
  brew = fd
  cargo = fd-find
  apt = fd-find
```

In the `fd-find` example, the package has different names:
- Homebrew: `fd`
- Cargo and APT: `fd-find`

### Mixed Format

Combine both formats in the same configuration:

```ccl
/= Simple format for consistent names
packages =
  = jq
  = wget

/= Source-specific for special cases
fd-find =
  brew = fd
  cargo = fd-find
  apt = fd-find

neovim =
  = brew
  = apt
  = pacman
```

## Configuration Examples

### Minimal Configuration

Track a few essential packages using default source priorities:

```ccl
/= Use system package manager preferences
sources =
  = brew
  = apt
  = cargo

/= Essential development tools
packages =
  = git
  = curl
  = wget
  = jq
```

### Multi-Platform Configuration

Configuration that works across macOS, Linux, and Windows:

```ccl
/= Platform package managers (Santa filters to available ones)
sources =
  = brew
  = scoop
  = apt
  = pacman
  = cargo
  = nix

/= Cross-platform packages
packages =
  = git
  = bat
  = ripgrep
  = fd
  = jq
  = curl

/= Platform-specific names
fd-find =
  brew = fd
  scoop = fd
  cargo = fd-find
  apt = fd-find
  pacman = fd
```

### Development Environment

Comprehensive setup for a development machine:

```ccl
/= Prefer system package manager, fallback to cargo
sources =
  = brew
  = apt
  = cargo

/= Version control
packages =
  = git
  = gh

/= Build tools
packages =
  = cmake
  = make

/= Languages and runtimes
packages =
  = rustup
  = nodejs
  = python3

/= CLI utilities
packages =
  = bat
  = ripgrep
  = fd
  = jq
  = fzf
  = exa

/= Special cases with different names
fd-find =
  brew = fd
  cargo = fd-find
  apt = fd-find

exa =
  brew = exa
  cargo = exa
  apt = exa
  pacman = exa
```

### Source-Specific Configuration

Install packages only from specific sources:

```ccl
/= Only use Homebrew
sources =
  = brew

/= Homebrew packages only
bat =
  = brew

ripgrep =
  = brew

neovim =
  = brew
```

## Environment Variables

Override configuration with environment variables:

### `SANTA_CONFIG`

Path to configuration file:

```bash
export SANTA_CONFIG=~/my-santa-config.ccl
santa status
```

### `SANTA_SOURCES_DIR`

Directory for source definitions:

```bash
export SANTA_SOURCES_DIR=~/.local/share/santa/sources
santa sources update
```

### `RUST_LOG`

Control logging (alternative to `-v` flags):

```bash
export RUST_LOG=santa=debug
santa status

export RUST_LOG=santa=trace
santa install
```

## Configuration Validation

Santa validates configuration on load. Common errors:

### Invalid CCL Syntax

```
Error: Failed to parse configuration
```

**Fix**: Check CCL syntax, ensure proper indentation, verify `=` placement.

### Unknown Source

```
Warning: Unknown source 'xyz' in configuration
```

**Fix**: Verify source name is valid. Available sources: `brew`, `cargo`, `apt`, `pacman`, `aur`, `scoop`, `nix`.

### Duplicate Package Definitions

If a package is defined multiple times, the last definition wins. Santa logs a warning:

```
Warning: Package 'ripgrep' defined multiple times
```

**Fix**: Consolidate package definitions.

## Advanced Configuration

### Layer System

Santa uses a data layer system for source definitions:

1. **Bundled layer**: Built-in source definitions
2. **Downloaded layer**: Definitions from `santa sources update`
3. **Custom layer**: Your local customizations in `~/.config/santa/sources/`

Later layers override earlier ones.

### Custom Source Definitions

Create custom package mappings in `~/.config/santa/sources/`:

```ccl
/= Custom source for internal packages
internal =
  type = custom
  command = internal-pm

/= Package mappings
myapp =
  internal = myapp-cli
```

Then use in your configuration:

```ccl
sources =
  = internal
  = brew

packages =
  = myapp
```

## Reference

### Complete Configuration Schema

```ccl
/= Optional comments describing the configuration

/= Package manager sources in priority order
sources =
  = source1
  = source2

/= Simple package list (same name across sources)
packages =
  = package1
  = package2

/= Source-specific package mappings
package3 =
  = source1
  = source2

/= Package with different names per source
package4 =
  source1 = name-in-source1
  source2 = name-in-source2
```

### Supported Package Managers

| Source | Platform | Notes |
|--------|----------|-------|
| `brew` | macOS, Linux | Homebrew package manager |
| `cargo` | All | Rust package manager |
| `apt` | Debian, Ubuntu | Debian package manager |
| `pacman` | Arch Linux | Arch package manager |
| `aur` | Arch Linux | Arch User Repository |
| `scoop` | Windows | Windows package manager |
| `nix` | All | Nix package manager |

## Next Steps

- See [User Guide](user-guide.md) for usage examples
- See [Troubleshooting](troubleshooting.md) for common issues
- Visit https://ccl.tylerbutler.com for complete CCL documentation

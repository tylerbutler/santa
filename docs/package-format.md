# Santa Package Format Reference

Santa uses the Categorical Configuration Language (CCL) format for defining packages and their available sources. This document describes the package database format and how to add new packages.

## Overview

The package database is stored in `crates/santa-cli/data/known_packages.ccl` and defines:
- Which packages are available
- Where each package can be installed from (brew, scoop, apt, pacman, nix, cargo, npm, pip, aur, flathub)
- Alternative package names for different package managers
- Pre-install hooks and installation overrides

## Package Database Structure

The file is organized into two sections:

### 1. Simple Format Packages

Packages that have the same name across all package managers use simple format:

```ccl
package-name =
  = source1
  = source2
  = source3
```

**Example:**
```ccl
fd =
  = brew
  = scoop
  = pacman
  = nix
```

This means `fd` is available from Homebrew, Scoop, Pacman, and Nix with the same package name.

### 2. Complex Format Packages

Packages with source-specific overrides (different names or special handling) use complex format:

```ccl
package-name =
  source1 = alternative-name
  source2 = alternative-name
  _sources =
    = source1
    = source2
```

**Example with Name Override:**
```ccl
ripgrep =
  brew = rg
  _sources =
    = scoop
    = pacman
    = nix
```

This means:
- When installing with Homebrew, the package name is `rg` (not `ripgrep`)
- When installing with Scoop, Pacman, or Nix, the package name is `ripgrep`

**Example with Pre-Install Hook:**
```ccl
oh-my-posh =
  aur = oh-my-posh-git
  brew =
    pre = brew tap jandedobbeleer/oh-my-posh
  scoop = https://github.com/JanDeDobbeleer/oh-my-posh/releases/latest/download/oh-my-posh.json
```

This means:
- AUR: install `oh-my-posh-git`
- Homebrew: first run `brew tap jandedobbeleer/oh-my-posh`, then install `oh-my-posh`
- Scoop: use custom JSON URL instead of standard package

**Example with Install Suffix:**
```ccl
@fluidframework/build-tools =
  npm =
    install_suffix = @latest
```

This means:
- When installing via npm, append `@latest` to install the latest version

## Available Package Sources

| Source | Platform | Package Manager | Install Command |
|--------|----------|-----------------|-----------------|
| brew | macOS | Homebrew | `brew install {package}` |
| scoop | Windows | Scoop | `scoop install {package}` |
| apt | Linux | APT | `sudo apt install {package}` |
| pacman | Linux | Pacman | `sudo pacman -Syyu {package}` |
| nix | macOS/Linux | Nix | `nix-env -iA nixpkgs.{package}` |
| cargo | Universal | Cargo | `cargo install {package}` |
| npm | Universal | NPM | `npm install -g {package}` |
| pip | Universal | pip | `pip install {package}` |
| aur | Linux | Arch User Repo | `paru/yay -S {package}` |
| flathub | Linux | Flatpak | `flatpak install flathub {package}` |

## Adding New Packages

### For Simple Packages (Same Name Everywhere)

1. Find the appropriate section in the file by category (Development Tools, Text Editors, etc.)
2. Add the package in alphabetical order within the section:

```ccl
new-package =
  = brew
  = scoop
  = pacman
  = nix
```

3. Include all sources where the package is available
4. Keep sources in consistent order: brew, scoop, apt, pacman, nix, cargo, npm, pip

### For Complex Packages (Different Names or Special Setup)

1. Add to the "Packages with complex format" section at the end
2. Include the source-specific overrides at the top level
3. List all sources in `_sources` section:

```ccl
my-package =
  brew = homebrew-name
  nix = nixpkgs-name
  _sources =
    = brew
    = nix
```

### For Aliases

Create alias packages that map short names to full package names:

```ccl
alias-name =
  brew = full-package-name
  scoop = full-package-name
  _sources =
    = brew
    = scoop
```

**Example:** The `rg` alias points to `ripgrep`:
```ccl
rg =
  scoop = ripgrep
  brew = ripgrep
  _sources =
    = apt
    = pacman
    = nix
```

## Package Organization

Packages are grouped by category using comments:

- Development Tools
- Text Editors
- File Management Tools
- Git Tools
- System Monitoring
- Data Processing
- Network Tools
- Disk Tools
- Shell Tools
- Database CLI Tools
- Security Tools

Keep new packages in the appropriate category for maintainability.

## Validation

Before committing changes, validate the package database:

```bash
cd /path/to/santa
python scripts/validate_data.py
```

This script checks for:
- ✗ Duplicate package names (errors)
- ✓ Invalid source references (errors)
- ⚠ Single-source packages (warnings)
- ℹ Alias references (informational)

All validation errors must be fixed before submitting a PR. Warnings should be reviewed but are acceptable.

## Best Practices

1. **Keep packages in alphabetical order** within each section
2. **Use consistent source order**: brew, scoop, apt, pacman, nix, cargo, npm, pip
3. **Add packages to appropriate category** - create new categories if needed
4. **Document special cases** - if a package needs pre-install hooks or custom URLs, add comments
5. **Validate before committing** - run `python scripts/validate_data.py`
6. **Test installation** - if possible, test the package installation from the new source

## Source Coverage Matrix

Current package availability:
- **Homebrew (brew)**: 77 packages
- **Scoop**: 61 packages
- **Pacman**: 61 packages
- **Nix**: 63 packages
- **APT**: 35 packages
- **Cargo**: 12 packages
- **NPM**: 11 packages
- **pip**: 3 packages

When adding new packages, prioritize Homebrew, Scoop, Pacman, and Nix for maximum cross-platform coverage.

## Examples

### Example 1: Simple Cross-Platform Tool

```ccl
/= File diff tool with consistent name across platforms
difftastic =
  = brew
  = scoop
  = pacman
  = nix
```

### Example 2: Tool with Platform-Specific Names

```ccl
/= Git integration tool with alternative names in some repos
github-cli =
  brew = gh
  _sources =
    = scoop
    = apt
    = pacman
    = nix
```

### Example 3: Alias to Full Package Name

```ccl
/= Short alias for ripgrep
rg =
  scoop = ripgrep
  brew = ripgrep
  _sources =
    = apt
    = pacman
    = nix
```

### Example 4: Package with Installation Suffix

```ccl
/= Framework package with version suffix
@fluidframework/build-tools =
  npm =
    install_suffix = @latest
```

## Related Files

- `crates/santa-cli/data/known_packages.ccl` - Main package database
- `crates/santa-cli/data/sources.ccl` - Source definitions with install commands
- `crates/santa-cli/data/santa-config.ccl` - Default user configuration
- `scripts/validate_data.py` - Database validation tool

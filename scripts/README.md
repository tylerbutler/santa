# Santa Scripts

Utility scripts for package discovery and analysis.

## Overview

This directory contains Python scripts used during Santa development to identify and catalog installable CLI tools across different package managers.

## Scripts

### collect-packages.py

Discovers and catalogs CLI tools with their installation methods across multiple package managers.

**Purpose**: Automates discovery of popular CLI tools and determines which package managers can install them.

**Features**:
- Scrapes awesome-cli-apps list for tool metadata
- Probes package manager availability (brew, apt, winget, scoop, choco, snap, nix)
- Tests installability of tools across available package managers
- Generates JSON catalog with installation commands
- Displays tabular summary of tool availability

**Usage**:

```bash
# Check first 50 tools (default)
python collect-packages.py

# Check specific number of tools
python collect-packages.py 100

# Check all discovered tools
python collect-packages.py 999999
```

**Output**:
- `cli_tools_with_installs.json` - Full catalog of tools with metadata and installation commands

**Requirements**:
- Python dependencies managed via `pyproject.toml` and `uv.lock`
- Install with: `uv sync` or `pip install -r requirements.txt`

## Development

### Setup

```bash
# Install dependencies with uv (recommended)
uv sync

# Or with pip
pip install requests beautifulsoup4 tabulate
```

### Project Structure

- `collect-packages.py` - Main collection script
- `pyproject.toml` - Python project configuration
- `uv.lock` - Locked dependencies for reproducible builds
- `ruff.toml` - Python linting configuration
- `justfile` - Task runner commands

### Tasks (via justfile)

```bash
# Run collection script
just collect

# Format and lint
just format
just lint
```

## Notes

- Scripts are used during **development** to populate Santa's package database
- Not required for runtime usage of Santa package manager
- Package manager detection adapts to available tools on the system
- Probing operations have timeout protection to avoid hanging

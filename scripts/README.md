# Santa Scripts

Utility scripts for package discovery, cross-referencing, and CCL generation.

## Overview

This directory contains Python scripts used during Santa development to identify, catalog, and cross-reference CLI tools across different package managers. The pipeline collects package data from multiple sources, scores packages by popularity, verifies availability, and generates CCL configuration for Santa.

## Quick Start

```bash
# Install dependencies
just sync

# Run full pipeline (collect → crossref → verify → generate)
just pipeline

# Or run individual steps
just collect-all      # Collect from all sources
just crossref         # Cross-reference and score
just verify           # Verify availability
just generate-ccl     # Generate CCL output
```

## Pipeline Overview

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Collect    │ →  │  Crossref   │ →  │   Verify    │ →  │ Generate    │
│  (sources)  │    │  (score)    │    │  (check)    │    │   (CCL)     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
     ↓                   ↓                  ↓                  ↓
 data/raw/*.json    crossref.json    verified.json    known_packages.ccl
```

## Data Sources

| Source | Type | Data Quality | Notes |
|--------|------|--------------|-------|
| **Homebrew** | API | Popularity ranks | 365-day install analytics |
| **Toolleeo** | CSV | Categories | 1,900+ curated CLI tools |
| **Modern Unix** | Markdown | Curated | ~30 modern CLI replacements |
| **Scoop** | GitHub | Manifests | Windows package manager |
| **AUR** | Metadata dump | Popularity & votes | Arch User Repository (102k+ packages) |
| **Arch** | pkgstats API | Install counts | Official Arch repos (30k+ packages) |
| **Awesome CLI Apps** | Markdown | Curated | 1,000+ tools |

## Scripts

### collect_all.py

Orchestrates collection from all package sources.

```bash
# Collect from all sources
uv run python collect_all.py

# Collect from specific sources
uv run python collect_all.py --sources homebrew toolleeo

# Collect from specific sources only
uv run python collect_all.py --sources homebrew toolleeo aur

# List available collectors
uv run python collect_all.py --list
```

**Output**: `data/raw/{source}.json` for each collector

### crossref_packages.py

Cross-references packages across sources and calculates popularity scores.

**Scoring Algorithm**:
- Homebrew rank: `max(0, 501 - rank)` points (top packages score highest)
- Modern Unix presence: +200 points (highly curated)
- Toolleeo presence: +50 points
- Awesome CLI Apps presence: +50 points
- Multi-source bonus: +25 points per additional source

```bash
# Cross-reference top 150 packages
uv run python crossref_packages.py --top 150

# Output all scored packages
uv run python crossref_packages.py --top 500
```

**Output**: `data/crossref_packages.json`

### verify_packages.py

Verifies package availability in target package managers (brew, scoop, apt, pacman, nix, cargo, npm).

```bash
# Verify top 100 packages
uv run python verify_packages.py --limit 100

# Verify all cross-referenced packages
uv run python verify_packages.py --limit 999
```

**Verification Methods**:
- **Homebrew**: API check at `formulae.brew.sh`
- **Scoop**: GitHub manifest existence check
- **apt/pacman/nix**: Static common package lists
- **cargo/npm**: Registry API checks

**Output**: `data/verified_packages.json`

### generate_ccl.py

Generates CCL format from verified packages, merging with existing `known_packages.ccl`.

```bash
# Preview merged output (stdout)
uv run python generate_ccl.py

# Write to source file
uv run python generate_ccl.py --write

# Require packages to be in multiple sources
uv run python generate_ccl.py --min-sources 2

# Dry run (show what would be written)
uv run python generate_ccl.py --write --dry-run
```

**Features**:
- Parses existing CCL preserving complex entries (pre/post hooks, name overrides)
- Merges new packages with existing data
- Outputs alphabetically sorted (simple entries first, then complex)
- Handles source-specific package name overrides

**Output**: `../crates/santa-cli/data/known_packages.ccl`

### models.py

Pydantic data models for package collection:

```python
class Package(BaseModel):
    name: str              # Normalized package name
    display_name: str      # Original display name
    source: str            # Source identifier (homebrew, toolleeo, etc.)
    source_id: str         # ID in source system
    popularity: int | None # Install count (if available)
    popularity_rank: int | None
    description: str | None
    homepage: str | None
    category: str | None
    collected_at: date
```

### collectors/

Individual collector implementations:

| File | Source | API/Method |
|------|--------|------------|
| `homebrew.py` | Homebrew | `formulae.brew.sh/api/analytics` |
| `toolleeo.py` | Toolleeo | GitHub raw CSV |
| `modern_unix.py` | Modern Unix | GitHub README parsing |
| `scoop.py` | Scoop | GitHub API (manifests) |
| `aur.py` | AUR | `aur.archlinux.org/packages-meta-v1.json.gz` |
| `arch.py` | Arch | `pkgstats.archlinux.de/api/packages` |
| `awesome_cli_apps.py` | Awesome CLI Apps | GitHub README parsing |

## Justfile Commands

```bash
just sync          # Install dependencies
just collect-all   # Collect from all sources
just collect-fast  # Collect quickly (all sources are fast now)
just collect homebrew toolleeo  # Collect specific sources
just crossref      # Cross-reference (default: top 150)
just crossref 200  # Cross-reference top 200
just verify        # Verify availability (default: 100)
just verify 50     # Verify top 50
just generate-ccl  # Generate CCL (min 2 sources)
just pipeline      # Run full pipeline
just format        # Format with Ruff
just lint          # Lint with Ruff
```

## Data Directory Structure

```
scripts/
├── data/
│   ├── raw/                    # Raw collector output
│   │   ├── homebrew.json
│   │   ├── toolleeo.json
│   │   ├── modern_unix.json
│   │   ├── scoop.json
│   │   ├── aur.json
│   │   ├── arch.json
│   │   └── awesome_cli_apps.json
│   ├── crossref_packages.json  # Cross-referenced & scored
│   └── verified_packages.json  # Verified availability
└── collectors/                 # Collector implementations
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `GITHUB_TOKEN` | GitHub API auth (higher rate limits) | None (60 req/hr) |

## Rate Limiting

- **GitHub API**: 60 req/hr unauthenticated, 5000/hr with `GITHUB_TOKEN`
- **AUR**: No rate limit (single metadata download)
- **Arch pkgstats**: 120 req/hr (built-in rate limiter)
- **Homebrew API**: No explicit limit (cached analytics)

## CCL Format Reference

Simple format (same name across sources):
```
bat =
  = brew
  = nix
  = pacman
  = scoop
```

Complex format (source-specific overrides):
```
ripgrep =
  brew = rg
  _sources =
    = nix
    = pacman
    = scoop
```

With pre/post hooks:
```
oh-my-posh =
  brew =
    pre = brew tap jandedobbeleer/oh-my-posh
  aur = oh-my-posh-git
```

## Development

```bash
# Setup
just sync

# Format and lint
just format
just lint

# Clean environment
just clean
```

## Notes

- Scripts are used during **development** to populate Santa's package database
- Not required for runtime usage of Santa
- Rate limiting protects against API abuse
- Collectors cache results in `data/raw/` for incremental updates

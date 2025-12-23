# Santa Development Guide

This guide covers development workflows, architecture decisions, and maintenance procedures for the Santa package manager project.

## Prerequisites

1. **Rust** (1.80 or later): Install from [rustup.rs](https://rustup.rs/)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Just** task runner: Install using [mise](https://mise.jdx.dev/)
   ```bash
   mise use -g just
   ```

3. **Git**: Verify it's installed
   ```bash
   git --version
   ```

## Quick Start

```bash
# Clone and setup
git clone https://github.com/tylerbutler/santa.git
cd santa
just setup

# Development workflow
just build      # Build debug version
just test       # Run tests
just lint       # Check code style
just fix        # Auto-fix issues
```

## Package Data Pipeline

Santa uses a **source-organized architecture** for package data, with automated discovery via external APIs and Repology for cross-platform name validation.

### Pipeline Overview

```
just pipeline
```

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────────┐
│ collect-packages│────▶│ crossref-packages│────▶│ build-repology-cache│
└─────────────────┘     └──────────────────┘     └─────────────────────┘
                                                           │
┌─────────────────┐     ┌──────────────────┐     ┌────────▼────────────┐
│  generate-index │◀────│   merge-verified │◀────│   validate-cached   │
└─────────────────┘     └──────────────────┘     └─────────────────────┘
```

### File Organization

```
crates/santa-cli/data/
├── sources/                      # SOURCE OF TRUTH (editable)
│   ├── brew.ccl                  # Homebrew packages
│   ├── apt.ccl                   # APT packages
│   ├── scoop.ccl                 # Scoop packages
│   ├── nix.ccl                   # Nix packages
│   ├── pacman.ccl                # Pacman packages
│   ├── aur.ccl                   # AUR packages
│   ├── cargo.ccl                 # Cargo crates
│   └── npm.ccl                   # NPM packages
│
├── discovery/                    # Pipeline intermediate data
│   ├── raw/                      # Raw API responses
│   ├── crossref_results.json     # Ranked package candidates
│   ├── repology_cache.json       # Cached Repology mappings
│   └── verified_packages.json    # Packages ready to merge
│
├── packages.ccl                  # Catalog (descriptions, verified status)
├── known_packages.ccl            # GENERATED INDEX (runtime)
└── sources.ccl                   # Package manager definitions
```

### Pipeline Stages

#### Stage 1: `collect-packages`

**Purpose:** Fetch raw package data from external sources

**Data Sources (APIs):**
- Homebrew analytics API (popularity/install counts)
- Scoop bucket listings
- AUR RPC API
- Curated lists (toolleeo CLI tools, modern-unix, awesome-cli-apps)

**Outputs:** `data/discovery/raw/*.json`

#### Stage 2: `crossref-packages --top=500`

**Purpose:** Cross-reference packages across sources and rank by popularity

**Inputs:**
- `data/discovery/raw/*.json` - Raw collected data
- `data/sources/*.ccl` - Existing packages (to filter duplicates)

**Logic:**
- Normalizes package names for matching
- Scores by: Homebrew rank, curated list presence, source count
- Filters packages already in source CCL files

**Outputs:** `data/discovery/crossref_results.json`

#### Stage 3: `build-repology-cache --from-crossref 200`

**Purpose:** Query Repology API to cache cross-platform name mappings

**Inputs:** `data/discovery/crossref_results.json` (top 200)

**Logic:**
- Queries [Repology API](https://repology.org/api) for each package (1 req/sec)
- Maps Repology repos → our sources (homebrew→brew, debian_12→apt, etc.)
- Caches results to avoid repeated API calls

**Outputs:** `data/discovery/repology_cache.json`

#### Stage 4: `validate-cached`

**Purpose:** Validate source entries against cached Repology data (fast, no API)

**Inputs:**
- `data/sources/*.ccl` - Package definitions
- `data/discovery/repology_cache.json` - Cached mappings
- `data/packages.ccl` - Catalog (skip already-verified)

**Logic:**
- Compares our name mappings against Repology's
- Identifies: OK, NOT_FOUND, MISMATCH, MISSING
- Updates catalog with `verified = YYYY-MM-DD`

**Outputs:** `data/packages.ccl` (updated verified timestamps)

#### Stage 5: `merge-verified`

**Purpose:** Merge verified packages into source CCL files

**Inputs:** `data/discovery/verified_packages.json`

**Outputs:**
- `data/sources/*.ccl` - New package entries
- `data/packages.ccl` - Descriptions from verified data

#### Stage 6: `generate-index`

**Purpose:** Generate unified runtime index from all sources

**Inputs:**
- `data/sources/*.ccl` - All source definitions
- `data/packages.ccl` - Catalog metadata

**Outputs:** `data/known_packages.ccl` - Runtime index

### Repology Tool

The `fetch-repology` binary provides direct Repology integration:

```bash
# Query a single package
just fetch-repology query ripgrep

# Build cache from top crossref packages
just fetch-repology build-cache --from-crossref 200

# Validate using cached data (fast)
just fetch-repology validate --from-cache

# Validate using live API (slow, 1 req/sec)
just fetch-repology validate brew apt

# See all options
just fetch-repology --help
```

### Manual Package Curation

Source files in `data/sources/*.ccl` can be edited directly:

**Simple packages:**
```ccl
/= Homebrew packages
bat =
fd =
```

**Name overrides (source name differs from canonical):**
```ccl
gh = github-cli
rg = ripgrep
```

**Complex config:**
```ccl
oh-my-posh =
  pre = brew tap jandedobbeleer/oh-my-posh
```

After editing, regenerate the index:
```bash
just generate-index
```

### Key Data Files

| File | Purpose |
|------|---------|
| `sources/*.ccl` | Per-source package definitions (editable) |
| `packages.ccl` | Catalog: descriptions, homepages, verified status |
| `known_packages.ccl` | Generated unified index for santa runtime |
| `discovery/crossref_results.json` | Ranked package candidates |
| `discovery/repology_cache.json` | Cached Repology name mappings |

### Testing & Validation

```bash
# Run all tests
just test

# Validate sources against Repology (cached)
just validate-cached

# Validate specific sources (live API)
just validate-sources brew apt

# Test specific package
cargo run -- install bat
```

## Key Principles

### Source Files are Truth
- `data/sources/*.ccl` files are manually curated
- These are checked into git and version controlled
- Edit these files to add/remove/modify packages

### Index is Generated
- `known_packages.ccl` is automatically generated
- Never edit this file manually
- Regenerate after any source file changes
- Can be regenerated anytime with `just generate-index`

### Separation of Concerns
- **Collection script** - Discovers packages, outputs CCL
- **Source files** - Human-editable, organized by manager
- **Index generator** - Merges sources into runtime format
- **Santa runtime** - Loads unified index for fast lookups

## Adding New Packages

### Method 1: Direct Edit (Recommended for few packages)

1. Edit the appropriate source file:
   ```bash
   vim crates/santa-cli/data/sources/brew.ccl
   ```

2. Add package in CCL format:
   ```ccl
   new-tool =
   ```

3. Regenerate index:
   ```bash
   just generate-index
   ```

4. Test:
   ```bash
   cargo run -- install new-tool
   ```

### Method 2: Collection Script (For bulk discovery)

1. Run collection script:
   ```bash
   cd scripts
   python3 collect-packages.py 100
   ```

2. Review generated files in `generated_sources/`

3. Copy desired packages to `../crates/santa-cli/data/sources/`

4. Regenerate index:
   ```bash
   cd ..
   just generate-index
   ```

## Code Quality

### Pre-commit Checklist
- [ ] `just check-all` passes
- [ ] `just test` passes
- [ ] `just generate-index` run after source changes
- [ ] No `unwrap()` or `todo!()` in production code
- [ ] Comprehensive error handling with context

### Security Guidelines
- All user inputs must be sanitized using `shell-escape` crate
- Script generation prevents command injection by design
- Never trust package names or user-provided strings
- Security tests validate injection prevention

## Project Structure

### Workspace Crates

**santa-cli** - Main CLI application
- Entry point and commands
- Script generation
- Package manager integrations

**santa-data** - Data models and CCL parser
- Reusable library
- CCL configuration parsing

**sickle** - CCL parser library
- General-purpose CCL parsing
- Serde support

### Build System

Uses `just` for task automation:
```bash
just                 # Show all commands
just build           # Debug build
just build-release   # Release build
just test            # Run tests
just test-coverage   # With coverage
just bench           # Benchmarks
just generate-index  # Regenerate package index
just docs            # Generate and open docs
```

## Release Process

Santa uses [release-plz](https://release-plz.dev/) for automated releases:

1. Changes merged to `main`
2. release-plz creates release PR with changelog
3. Merge release PR triggers:
   - Version bumps
   - Git tags
   - cargo-dist builds binaries
   - GitHub release creation
   - crates.io publication

## Performance

Santa is designed for high performance:
- **67-90% faster** than sequential package operations
- **Async I/O** with tokio for non-blocking operations
- **Professional caching** via moka with TTL and LRU eviction
- **Memory efficient** with zero-copy string handling

Run benchmarks:
```bash
just bench
```

## Troubleshooting

### Index out of sync
```bash
just generate-index
```

### Tests failing after package changes
```bash
just generate-index
just test
```

### CCL parsing errors
Check for:
- Proper `package =` format in source files
- No unescaped special characters
- Comments use `/=` prefix

## Resources

- **User Guide:** [santa-cli README](crates/santa-cli/README.md)
- **CLI Reference:** [docs/cli-reference.md](docs/cli-reference.md)
- **API Docs:** [docs.rs/santa-data](https://docs.rs/santa-data) | [docs.rs/sickle](https://docs.rs/sickle)
- **CCL Format:** [ccl.tylerbutler.com](https://ccl.tylerbutler.com)
- **Project Docs:** [CLAUDE.md](CLAUDE.md)

## Contributing

We welcome contributions! Please:
1. Create a feature branch
2. Make your changes
3. Run `just check-all`
4. Add tests for new functionality
5. Update documentation as needed
6. Submit PR with clear description

See the main [README.md](README.md) for more details.

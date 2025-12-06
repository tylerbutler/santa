# Santa Development Guide

This guide covers development workflows, architecture decisions, and maintenance procedures for the Santa package manager project.

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

Santa uses a **source-organized architecture** for package data, separating maintainable source files from the runtime index.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Package Data Pipeline                     │
└─────────────────────────────────────────────────────────────┘

1. COLLECTION (Optional - discover new packages)
   scripts/collect-packages.py
   └─> generated_sources/*.ccl

2. CURATION (Manual - review and edit)
   Review generated files
   Add overrides/configs
   └─> crates/santa-cli/data/sources/*.ccl

3. INDEX GENERATION (Build step)
   just generate-index
   └─> crates/santa-cli/data/known_packages.ccl

4. RUNTIME (What santa uses)
   Santa loads known_packages.ccl at runtime
```

### File Organization

```
santa/
├── scripts/
│   └── collect-packages.py           # Package discovery tool
│
├── crates/santa-cli/
│   ├── data/
│   │   ├── sources/                  # SOURCE OF TRUTH (editable)
│   │   │   ├── brew.ccl              # Homebrew packages
│   │   │   ├── scoop.ccl             # Scoop packages
│   │   │   ├── pacman.ccl            # Pacman packages
│   │   │   ├── nix.ccl               # Nix packages
│   │   │   ├── cargo.ccl             # Cargo crates
│   │   │   ├── npm.ccl               # NPM packages
│   │   │   ├── apt.ccl               # APT packages
│   │   │   └── aur.ccl               # AUR packages
│   │   │
│   │   ├── known_packages.ccl        # GENERATED INDEX (runtime)
│   │   └── known_packages.ccl.old    # Original format (reference)
│   │
│   └── src/
│       └── bin/
│           └── generate_index.rs     # Index generator
```

### Workflow Details

#### 1. Package Discovery (Optional)

Discover new packages using the collection script:

```bash
cd scripts
python3 collect-packages.py 50  # Check first 50 tools
```

**Output:**
- `cli_tools_with_installs.json` - Analysis data
- `generated_sources/*.ccl` - Generated CCL files per manager

**Note:** The collection script uses isolated CCL writing functions marked with:
```python
# ============================================================================
# CCL Writing Functions (TODO: Replace with proper sickle library integration)
# ============================================================================
```
This allows future replacement with proper sickle library integration.

#### 2. Package Curation (Manual)

Review and edit source files in `crates/santa-cli/data/sources/`:

**Simple packages:**
```ccl
/= Homebrew packages
bat =
fd =
ripgrep = rg
```

**Packages with overrides:**
```ccl
github-cli = gh              # Name override
go = golang                  # Different name in brew
```

**Packages with complex config:**
```ccl
oh-my-posh =
  pre = brew tap jandedobbeleer/oh-my-posh
```

#### 3. Index Generation (Required)

After editing source files, regenerate the index:

```bash
just generate-index
```

This runs the Rust binary at `crates/santa-cli/src/bin/generate_index.rs` which:
- Reads all `data/sources/*.ccl` files
- Filters comment lines (keys starting with `/`)
- Merges packages across sources
- Separates simple packages from those with overrides
- Writes `data/known_packages.ccl`

**Generated index format:**
```ccl
/= Generated package index
/= DO NOT EDIT - Generated from data/sources/*.ccl
/= Run: just generate-index to regenerate

/= Packages with simple format (no source-specific overrides)
bat =
  = brew
  = scoop
  = pacman

/= Packages with complex format (have source-specific overrides)
github-cli =
  brew = gh
  _sources =
    = apt
    = scoop
```

#### 4. Testing & Validation

```bash
# Run all tests
just test

# Run specific package
cargo run -- install bat

# Lint and fix
just lint
just fix
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

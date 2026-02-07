# Development Guide

## Prerequisites

- **Rust** 1.80+ - https://rustup.rs/
- **just** - Task runner - https://github.com/casey/just
- **Python 3** - For config generation scripts and package collection tools
- **ruff** (optional) - Python formatter for scripts/

### Optional Tools

- **cargo-nextest** - Faster parallel test runner
- **cargo-llvm-cov** - Code coverage
- **cargo-audit** + **cargo-deny** - Security auditing
- **cargo-watch** - File watcher for development

## Building

```bash
just build            # Debug build (alias: b)
just build-release    # Release build (alias: br)
```

This is a workspace with three crates. To build a specific crate:

```bash
cargo build -p santa        # CLI binary
cargo build -p santa-data   # Data models library
cargo build -p sickle       # CCL parser library
```

## Testing

```bash
just test             # Run all tests (alias: t)
just test-fast        # Run with nextest for parallel execution (alias: tf)
just test-all         # Run with all features enabled (alias: ta)
just test-unit        # Unit tests only
just test-integration # Integration tests only
just test-watch       # Watch mode
```

### Coverage

```bash
just test-coverage    # Generate lcov.info + HTML report
just coverage-report  # Generate HTML report from last run
```

### CCL Test Suite

Santa integrates with the shared ccl-test-data JSON test suite:

```bash
just download-ccl-tests  # Download test data from latest release
just test-ccl            # Run CCL test suites with detailed results
```

## Linting

```bash
just lint             # Run clippy (alias: l)
just fix              # Auto-fix formatting and lint issues (alias: f)
```

## Formatting

```bash
just format           # Format Rust + Python code
just format --check   # Check formatting without changes
```

Note: `just format` also runs `ruff format scripts/` for Python files.

## Running All Checks

```bash
just pr               # Full PR checks: format, docs, lint, configs, coverage, audit, build, verify-package
just main             # Main branch checks: format, docs, lint, coverage, audit, release build
```

## CI

CI workflows mirror the tiered system from the rust-template:

- **Tier 1**: Basic CI (test, lint, format, docs)
- **Tier 2**: + coverage, release-plz, commit-lint
- **Tier 3**: + cross-platform testing, binary distribution

### Configuration Management

Commit types are managed via `commit-types.json` (single source of truth):

```bash
just generate-configs  # Regenerate cliff.toml and .commitlintrc.json
just check-configs     # Verify configs are in sync
```

## Release Process

Releases are automated via [release-plz](https://release-plz.ieni.dev/) and [cargo-dist](https://opensource.axo.dev/cargo-dist/):

1. Commits pushed to `main` trigger release-plz to create/update a release PR
2. Merging the release PR creates git tags for each crate
3. Tags trigger cargo-dist to build binaries and publish to crates.io

### Crate Publishing Order

Crates must be published in dependency order:
1. `sickle` (no internal deps)
2. `santa-data` (depends on sickle)
3. `santa` (depends on santa-data)

## Package Data Pipeline

Santa includes a multi-stage pipeline for discovering and curating package data. See [DEVELOPMENT.md](DEVELOPMENT.md) for the full pipeline documentation.

Quick reference:

```bash
just pipeline           # Run full discovery pipeline
just generate-index     # Regenerate package index from sources
just collect-packages   # Fetch raw package data from APIs
just validate-cached    # Validate sources against Repology cache
```

## Additional Commands

```bash
just clean            # Clean build artifacts
just audit            # Security audit (alias: a)
just docs             # Generate and open documentation
just docs-check       # Check docs build without warnings
just bench            # Run benchmarks
just markdown-help    # Generate CLI markdown reference
just semver           # Check for semver-incompatible changes
just verify-package   # Verify crates can be packaged
just sickle-capabilities  # Generate sickle capabilities docs
```

## Project Structure

See [ARCHITECTURE.md](ARCHITECTURE.md) for the crate structure, module responsibilities, and design decisions.

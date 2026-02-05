# Santa Workspace 🎅

**A modern, high-performance package manager meta-tool ecosystem built in Rust.**

This repository contains the Santa package manager and its supporting libraries. Santa helps developers install and manage packages across multiple platforms and package managers with a single command.

[![Build Status](https://github.com/tylerbutler/santa/actions/workflows/test.yml/badge.svg)](https://github.com/tylerbutler/santa/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/tylerbutler/santa/branch/main/graph/badge.svg)](https://codecov.io/gh/tylerbutler/santa)
[![MemBrowse](https://membrowse.com/badge.svg)](https://membrowse.com/public/tylerbutler/santa)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Workspace Structure

This is a Cargo workspace containing three packages:

### 📦 [santa-cli](crates/santa-cli)

The main Santa command-line application. Install and manage packages across Homebrew, Cargo, APT, Pacman, Scoop, Nix, and more.

**For users:** See the [User Guide](docs/user-guide.md) for complete usage instructions.

```bash
# Install Santa
cargo install santa-cli

# Check package status
santa status

# Install missing packages (generates script)
santa install

# Review and run the generated script
sh ~/.santa/scripts/install_*.sh
```

### Quick Examples

```bash
# Add packages to your tracking list
santa add ripgrep cargo
santa add bat brew

# Check what's installed
santa status --all

# Update source definitions
santa sources update

# Generate installation script for specific source
santa install brew

# Direct execution mode (use with caution)
santa install -x
```

### 📊 [santa-data](crates/santa-data)

Core data models, schemas, and CCL configuration parser for Santa. Reusable library for tools that need to work with Santa's configuration format.

```rust
use santa_data::parser::parse_ccl_config;

let config = parse_ccl_config(ccl_string)?;
```

### 🔧 [sickle](crates/sickle)

A robust Rust parser for CCL (Categorical Configuration Language) with Serde support. General-purpose CCL parsing library.

```rust
use sickle::{parse, from_str};

let model = parse(ccl_string)?;
let config: MyConfig = from_str(ccl_string)?;
```

## Quick Links

- **User Documentation:** [User Guide](docs/user-guide.md) | [Configuration Guide](docs/configuration.md) | [Troubleshooting](docs/troubleshooting.md)
- **API Documentation:** [docs.rs/santa-data](https://docs.rs/santa-data) | [docs.rs/sickle](https://docs.rs/sickle)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)

## Key Features

### 🚀 Performance
- **67-90% faster** concurrent package operations via async execution
- **Professional-grade caching** with TTL and LRU eviction
- **Memory efficient** with zero unnecessary allocations

### 🛡️ Quality
- **144 comprehensive tests** with >90% code coverage across all crates
- **Security hardening** with input sanitization and injection protection
- **Production-ready** error handling and structured logging

### 🔧 Architecture
- **Zero `unwrap()` and `todo!()`** in production code
- **Strong typing** with builder patterns and validation
- **Cross-platform** support for Linux, macOS, and Windows
- **Modular design** with reusable libraries

## Development

Santa uses [`just`](https://github.com/casey/just) for development workflow automation.

### Quick Start

```bash
# Clone repository
git clone https://github.com/tylerbutler/santa.git
cd santa

# Install development tools
just setup

# Run quick checks
just check-quick

# Run all tests
just test

# Development with hot reload
just dev
```

### Essential Commands

| Command | Description |
|---------|-------------|
| `just` | Show all available commands |
| `just build` | Build all workspace crates |
| `just test` | Run all tests across workspace |
| `just lint` | Run clippy linting |
| `just check-all` | Run complete pre-commit checks |
| `just docs` | Generate and open documentation |

### Testing

```bash
# Run all tests
just test

# Run with coverage
just test-coverage

# Fast parallel testing
just test-fast

# Run specific crate tests
cd crates/santa-cli && cargo test
cd crates/santa-data && cargo test
cd crates/sickle && cargo test
```

### Benchmarking

```bash
# Run all benchmarks
just bench

# Save baseline
just bench-baseline my-feature

# Compare against baseline
just bench-compare my-feature
```

### Code Quality

```bash
# Check code style
just check-style

# Auto-fix issues
just fix

# Security audit
just audit

# Check dependencies
just deps
```

## Project Structure

```
santa/
├── crates/
│   ├── santa-cli/      # Main CLI application
│   │   ├── src/
│   │   ├── tests/
│   │   └── Cargo.toml
│   ├── santa-data/     # Data models and CCL parser
│   │   ├── src/
│   │   └── Cargo.toml
│   └── sickle/         # CCL parser library
│       ├── src/
│       ├── tests/
│       ├── examples/
│       └── Cargo.toml
├── data/               # Package data and definitions
├── templates/          # Script generation templates
├── scripts/            # Development and analysis scripts
├── justfile            # Development task runner
├── Cargo.toml          # Workspace configuration
└── README.md           # This file
```

## Architecture Overview

### Configuration System
- **CCL Format** - Modern configuration language (via sickle)
- **Hot-Reloading** - Real-time configuration updates
- **Environment Overrides** - All settings configurable via env vars
- **Migration Support** - Transparent YAML-to-CCL migration

### Package Management
- **Multi-Source** - Support for 7+ package managers
- **Async Operations** - High-performance concurrent execution
- **Intelligent Caching** - Reduces redundant operations
- **Cross-Platform** - Works on Linux, macOS, Windows

### Script Generation
- **Safe-by-Default** - Generates scripts instead of direct execution
- **Platform-Specific** - Shell (.sh), PowerShell (.ps1), Batch (.bat)
- **Template-Driven** - Uses Tera templating engine
- **Security-First** - Input sanitization prevents injection

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

## Contributing

We welcome contributions! Please follow these guidelines:

### Development Setup

1. **Install Rust** (1.80+): https://rustup.rs/
2. **Install Just**: `cargo install just`
3. **Clone and setup**:
   ```bash
   git clone https://github.com/tylerbutler/santa.git
   cd santa
   just setup
   ```

### Code Standards

- **Zero `unwrap()` and `todo!()`** in production code
- **Comprehensive error handling** with context
- **Tests required** for all new functionality
- **Documentation required** for public APIs
- **Security-first** approach to all changes

### Pull Request Process

1. Create a feature branch: `git checkout -b feature/my-feature`
2. Make your changes
3. Run checks: `just check-all`
4. Add tests for new functionality
5. Update documentation as needed
6. Submit PR with clear description

### Testing Requirements

- Unit tests for new functionality
- Integration tests for user-facing features
- Property-based tests for parsers and data structures
- Benchmarks for performance-critical changes

## CI/CD

Santa uses GitHub Actions for continuous integration:

- **Multi-platform testing** (Linux, macOS, Windows)
- **Comprehensive test suite** with coverage reporting via Codecov
- **Security auditing** and dependency checking
- **Performance regression testing**
- **Automated releases** with cross-platform binaries via cargo-dist

Run the same checks locally:

```bash
# Run CI checks
just ci

# Platform-specific CI
just ci-linux
just ci-macos
just ci-windows
```

## Release Process

Santa uses [release-plz](https://release-plz.dev/) for automated releases:

1. Changes are merged to `main`
2. release-plz creates release PRs with updated changelogs
3. Merging the release PR triggers:
   - Version bumps
   - Git tags
   - cargo-dist builds cross-platform binaries
   - GitHub release creation
   - crates.io publication

## Performance

Santa is designed for high performance:

- **67-90% faster** than sequential package operations
- **Async I/O** with tokio for non-blocking operations
- **Professional caching** via moka with TTL and LRU eviction
- **Memory efficient** with zero-copy string handling where possible

### Benchmarks

Results from criterion benchmarks on typical workloads:

- Package status checking: 70-90% faster than sequential
- Concurrent installations: 67-85% faster than sequential
- Configuration parsing: <1ms for typical configs

Run benchmarks:

```bash
just bench
```

## Documentation

### User Documentation
- **[User Guide](docs/user-guide.md)** - Installation, commands, and workflows
- **[Configuration Guide](docs/configuration.md)** - CCL format and configuration options
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

### Developer Documentation
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development setup and guidelines
- **API Docs:** [docs.rs/santa-data](https://docs.rs/santa-data) | [docs.rs/sickle](https://docs.rs/sickle)
- **[CLAUDE.md](CLAUDE.md)** - Project context and architecture

Generate local documentation:

```bash
just docs
```

## License

All packages in this workspace are licensed under the [MIT License](LICENSE).

## Acknowledgments

- Built with the modern Rust ecosystem
- Inspired by cross-platform package management needs
- Uses CCL (Categorical Configuration Language) for configuration
- Thanks to all contributors and the open source community

---

**Made with ❤️ by Tyler Butler and contributors**

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/tylerbutler/santa/compare/v0.1.0...v0.1.1) - 2025-11-15

### Added

- modernize architecture with security fixes and release automation ([#1](https://github.com/tylerbutler/santa/pull/1))

### Other

- Update known packages
- Fix sources
- Fix prepending and add flathub
- CI updates
- Use checkout v4 action
- Debug build
- Use checkout v3 action
- Lint fixes/ignores
- Formatting
- CI
- lint fixes
- Add support for executing install commands
- CI
- CI
- CI
- CI
- justfile
- Replace "Elf" term/concept.

### Added
- Initial release of Santa package manager
- CCL-based configuration system with hot-reloading
- Script generation for safe command execution
- Multi-platform support (Linux, macOS, Windows)
- Package manager detection and abstraction
- Comprehensive error handling with `anyhow` and `thiserror`
- Security-focused design preventing command injection

### Changed
- Migrated from HOCON to CCL configuration format
- Refactored to script generation model instead of direct execution

## [0.1.0] - 2024-09-15

### Added
- Initial project setup
- Basic package manager abstraction
- Command-line interface with `clap`
- Configuration file support
- Testing infrastructure

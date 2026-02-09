# Changelog

All notable changes to this project will be documented in this file.

## [santa-v0.3.3] - 2026-02-09

No notable changes in this release.
## [santa-v0.3.2] - 2026-01-31

### Documentation

- Remove stale dates and stats from CLAUDE.md (#72)

## [santa-v0.3.1] - 2026-01-18

### Bug Fixes

- Resolve clippy dead code warnings in schemas.rs


### Features

- Package discovery pipeline with Repology integration (#46)

## [santa-v0.3.0] - 2025-12-16

### Features

- Add --markdown-help flag for documentation generation (#39)

- Display data location and use config directory (#42)

- Reorganize package data by source (#45)


### Performance

- Replace ureq and tera with lighter alternatives (#47)

## [santa-v0.2.0] - 2025-12-01

### Bug Fixes

- Add 32-bit architecture support and remove 32-bit Windows CI targets (#34)

- Resolve source-specific package names in status check (#31)

## [santa-v0.1.4] - 2025-11-17

No notable changes in this release.
## [santa-v0.1.3] - 2025-11-17

### Features

- Enhance workspace configuration and CI for multi-package best practices (#23)

## [santa-v0.1.2] - 2025-11-17

### Bug Fixes

- Make source system extensible without code changes (#19)

## [santa-v0.1.1] - 2025-11-17

### Bug Fixes

- Add Windows package manager to default config sources (#8)

- Add version requirement for santa-data dependency

- Correct README path in santa-cli Cargo.toml (#9)

- Move templates into santa-cli crate for cargo packaging


# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

No notable changes in this release.

## [santa-v0.3.0] - 2025-12-16

### Documentation

- Add comprehensive user documentation (#58)


### Features

- Add --markdown-help flag for documentation generation (#39)

- Display data location and use config directory (#42)

- Reorganize package data by source (#45)


### Performance

- Optimize release binary size and add size tracking (#48)

- Replace ureq and tera with lighter alternatives (#47)


## [santa-v0.2.0] - 2025-12-01

### Bug Fixes

- Add 32-bit architecture support and remove 32-bit Windows CI targets (#34)

- Resolve source-specific package names in status check (#31)


## [santa-v0.1.4] - 2025-11-17

No notable changes in this release.

## [santa-v0.1.3] - 2025-11-17

### Bug Fixes

- Disable GitHub release creation in release-plz (#22)


### Features

- Enhance workspace configuration and CI for multi-package best practices (#23)


## [santa-v0.1.2] - 2025-11-17

### Bug Fixes

- Use RELEASE_PLZ_TOKEN to trigger release CI

- Make source system extensible without code changes (#19)


## [santa-v0.1.1] - 2025-11-17

### Bug Fixes

- Add platform-aware source filtering to default config (#4)

- Add Windows package manager to default config sources (#8)

- Add version requirement for santa-data dependency

- Correct README path in santa-cli Cargo.toml (#9)

- Use RELEASE_PLZ_TOKEN to trigger CI on release PRs (#15)

- Move templates into santa-cli crate for cargo packaging


### Features

- Modernize architecture with security fixes and release automation (#1)



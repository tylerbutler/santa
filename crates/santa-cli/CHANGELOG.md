# Changelog

All notable changes to this project will be documented in this file.

### Features

- Add --markdown-help flag for documentation generation (#39)

- Display data location and use config directory (#42)

- Reorganize package data by source (#45)


### Performance

- Replace ureq and tera with lighter alternatives (#47)


### Refactoring

- Remove serde_yaml dependency and migration module (#50)


### Testing

- Add tests (#60)

- Add E2E test suite and fix SANTA_CONFIG_PATH handling (#54)


### Bug Fixes

- Add 32-bit architecture support and remove 32-bit Windows CI targets (#34)

- Resolve source-specific package names in status check (#31)


### Features

- Enhance workspace configuration and CI for multi-package best practices (#23)


### Bug Fixes

- Make source system extensible without code changes (#19)


### Bug Fixes

- Move templates into santa-cli crate for cargo packaging


### Bug Fixes

- Add version requirement for santa-data dependency

- Correct README path in santa-cli Cargo.toml (#9)


### Refactoring

- Remove unused dependencies and fix clippy warnings (#12)

- Remove unused code (#13)


### Bug Fixes

- Add Windows package manager to default config sources (#8)


### Refactoring

- Migrate to workspace structure with CCL-only configuration (#5)

- Migrate to workspace structure with santa-data crate (#6)


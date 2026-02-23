# Changelog

All notable changes to this project will be documented in this file.

## v0.2.0 - 2026-02-23


### Added

- Add `from_str_with_options` API for serde deserialization with custom ParserOptions

### Fixed

- Allow special characters in CCL map keys during build_hierarchy
- Use lexical sort instead of reverse for duplicate keys in reference_compliant mode

## v0.1.3 - 2026-02-05

Baseline version established for changie migration.

## [sickle-v0.1.3] - 2026-01-31

### Bug Fixes

- Support panic=abort in downstream crates (#75)

## [sickle-v0.1.2] - 2026-01-18

### Bug Fixes

- Support Vec<Struct> serde roundtrip serialization

### Features

- Add configurable parser options (#53)

## [sickle-v0.1.1] - 2025-12-16

### Bug Fixes

- Parser cleanup and test improvements (#56)

- Filter out comments in filter validation tests (#61)

### Features

- Add granular feature flags and serde serialization (#44)

## [sickle-v0.1.0] - 2025-12-01

### Features

- Add CCL parser library and codecov tracking (#26)


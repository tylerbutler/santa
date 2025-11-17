# Santa Data

**Data models, schemas, and CCL configuration parser for the Santa package manager.**

This library provides the core data structures and configuration parsing functionality used by Santa. It's designed to be reusable for other tools that need to work with Santa's configuration format or data models.

[![crates.io](https://img.shields.io/crates/v/santa-data.svg)](https://crates.io/crates/santa-data)
[![Documentation](https://docs.rs/santa-data/badge.svg)](https://docs.rs/santa-data)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **CCL Configuration Parser** - Parse Santa's CCL-based configuration files
- **Data Models** - Strongly-typed models for packages, sources, and configuration
- **Serde Integration** - Full serialization/deserialization support
- **Validation** - Built-in validation for configuration data
- **Builder Patterns** - Ergonomic builders for constructing data models

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
santa-data = "0.1"
```

## Usage

### Parse CCL Configuration

```rust
use santa_data::parser::parse_ccl_config;

let ccl = r#"
sources =
  = brew
  = cargo
  = apt

packages =
  = ripgrep
  = bat
  = fd
"#;

let config = parse_ccl_config(ccl)?;
println!("Sources: {:?}", config.sources);
println!("Packages: {:?}", config.packages);
```

### Work with Data Models

```rust
use santa_data::models::{PackageSource, Package};

// Define package sources
let sources = vec![
    PackageSource::Brew,
    PackageSource::Cargo,
    PackageSource::Apt,
];

// Create a package reference
let package = Package::new("ripgrep");
```

### Use Configuration Builder

```rust
use santa_data::config::ConfigBuilder;

let config = ConfigBuilder::default()
    .sources(vec!["brew", "cargo"])
    .packages(vec!["ripgrep", "bat"])
    .cache_ttl_seconds(3600)
    .build()?;
```

### Validation

```rust
use santa_data::config::Config;
use validator::Validate;

let config = Config::from_ccl(ccl_string)?;
config.validate()?; // Validates sources, packages, cache settings
```

## Data Models

### Core Types

- **`Config`** - Main configuration structure with sources, packages, and settings
- **`PackageSource`** - Enumeration of supported package managers (Brew, Cargo, APT, etc.)
- **`Package`** - Package reference with optional metadata
- **`CacheConfig`** - Cache settings (TTL, size limits)

### Configuration Schema

The library provides JSON Schema support for validation and documentation:

```rust
use santa_data::schemas::generate_config_schema;

let schema = generate_config_schema();
println!("{}", serde_json::to_string_pretty(&schema)?);
```

## CCL Format

Santa Data uses CCL (Categorical Configuration Language) for configuration files. CCL is a simple, indentation-based format:

```ccl
/= This is a comment

/= List items use empty keys with indentation
sources =
  = brew
  = cargo
  = apt

packages =
  = ripgrep
  = bat
  = fd

/= Nested configuration
cache =
  ttl_seconds = 3600
  max_size = 1000
```

## Integration with Sickle

Santa Data uses the [sickle](https://crates.io/crates/sickle) crate for CCL parsing, which provides:

- Pure Rust CCL parser
- Serde integration
- Memory-efficient parsing
- Comprehensive error reporting

## API Documentation

For complete API documentation, see [docs.rs/santa-data](https://docs.rs/santa-data).

### Key Modules

- **`models`** - Core data structures (Config, Package, PackageSource)
- **`parser`** - CCL parsing utilities
- **`config`** - Configuration builders and loaders
- **`schemas`** - JSON Schema generation

## Examples

See the [examples](examples/) directory for complete usage examples:

- `parse_config.rs` - Parse CCL configuration files
- `build_config.rs` - Build configuration programmatically
- `validate_config.rs` - Validate configuration data

Run examples with:

```bash
cargo run --example parse_config
```

## Development

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_ccl_config
```

### Documentation

```bash
# Generate and open docs
cargo doc --open

# Check docs for warnings
cargo doc --no-deps
```

## Contributing

Contributions are welcome! This library is part of the [Santa workspace](https://github.com/tylerbutler/santa).

Please ensure:
- Tests pass: `cargo test`
- Code is formatted: `cargo fmt`
- Lints pass: `cargo clippy`

## License

Licensed under the [MIT License](LICENSE).

## Related Crates

- [`santa-cli`](../santa-cli) - Command-line interface for Santa
- [`sickle`](../sickle) - CCL parser used by Santa Data

---

Part of the [Santa package manager](https://github.com/tylerbutler/santa) project.

# Sickle

A robust Rust parser for **CCL** (Categorical Configuration Language) with optional Serde support.

## Features

- **Pure Rust implementation** - Zero unsafe code
- **Two API styles** - Direct `Model` navigation or Serde deserialization
- **Complete CCL support** - Lists, nested records, multiline values, comments
- **Memory efficient** - Optional string interning via feature flag
- **Well-tested** - Comprehensive test suite with property-based tests

## Quick Start

### Direct API

```rust
use sickle::parse;

let ccl = r#"
name = Santa
version = 0.1.0
author = Tyler Butler
"#;

let model = parse(ccl)?;
assert_eq!(model.get("name")?.as_str()?, "Santa");
assert_eq!(model.get("version")?.as_str()?, "0.1.0");
```

### Serde Integration

```rust
use serde::Deserialize;
use sickle::from_str;

#[derive(Deserialize)]
struct Config {
    name: String,
    version: String,
    author: String,
}

let ccl = r#"
name = Santa
version = 0.1.0
author = Tyler Butler
"#;

let config: Config = from_str(ccl)?;
assert_eq!(config.name, "Santa");
```

## CCL Syntax

CCL uses simple key-value pairs with indentation for nesting:

```ccl
/= This is a comment

name = MyApp
version = 1.0.0

/= Lists use empty keys
dependencies =
  = tokio
  = serde
  = clap

/= Nested configuration
database =
  host = localhost
  port = 5432
  credentials =
    username = admin
    password = secret
```

## License

MIT

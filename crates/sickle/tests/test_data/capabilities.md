# Sickle Capabilities Guide

This document provides a comprehensive overview of all CCL functions and parser behaviors supported by sickle.

> **Quick Reference**: See [../../data/ccl-registry.toml](../../data/ccl-registry.toml) for the machine-readable registry.
>
> **Note**: "Features" in test data are grouping labels only. Filter on **functions** and **behaviors**.

## Table of Contents

- [Model API](#model-api)
- [Parser Features](#parser-features)
- [Value Interpretation](#value-interpretation)
- [Error Handling](#error-handling)
- [Cargo Features](#cargo-features)
- [Test Coverage](#test-coverage)
- [Roadmap](#roadmap)

---

## Model API

The `Model` type provides the core API for navigating parsed CCL documents. All methods are defined in `src/model.rs`.

### Navigation Methods

| Method | Description | Example |
|--------|-------------|---------|
| `get(key)` | Get value by key from a map | `model.get("name")` |
| `at(path)` | Get nested value using dot path | `model.at("database.host")` |

### Type Conversion Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `as_str()` | Convert singleton to string | `Result<&str>` |
| `as_list()` | Convert to list slice | `Result<&[Model]>` |
| `as_map()` | Convert to map reference | `Result<&BTreeMap<String, Model>>` |
| `parse_value<T>()` | Parse singleton to any `FromStr` type | `Result<T>` |

### Type Checking Methods

| Method | Description | Returns |
|--------|-------------|---------|
| `is_singleton()` | Check if value is a singleton | `bool` |
| `is_list()` | Check if value is a list | `bool` |
| `is_map()` | Check if value is a map | `bool` |

### Composition Methods

| Method | Description | Behavior |
|--------|-------------|----------|
| `merge(other)` | Merge two models | Map+Map=merged, List+List=concat, Singleton+Singleton=list |

### Usage Examples

```rust
use sickle::parse;

// Navigation
let model = parse("name = Santa\nversion = 1.0")?;
let name = model.get("name")?.as_str()?;

// Nested access
let ccl = "database = host = localhost\nport = 5432";
let model = parse(ccl)?;
let host = model.at("database.host")?.as_str()?;

// Typed parsing
let ccl = "port = 5432\nenabled = true";
let model = parse(ccl)?;
let port: u16 = model.get("port")?.parse_value()?;
let enabled: bool = model.get("enabled")?.parse_value()?;

// List access
let ccl = "items =\n  = one\n  = two\n  = three";
let model = parse(ccl)?;
let items = model.get("items")?.as_list()?;
for item in items {
    println!("{}", item.as_str()?);
}
```

---

## Parser Features

### Supported Syntax ✅

#### Basic Key-Value Pairs
```ccl
name = Santa
version = 1.0.0
```

#### Multiline Values
```ccl
description = This is a long
  description that spans
  multiple lines
```

#### Comments
```ccl
/= This is a comment
name = Santa
/ This is also a comment
version = 1.0
```

#### Empty Key Lists
```ccl
items =
  = one
  = two
  = three
```

#### Nested Structures
```ccl
database =
  host = localhost
  port = 5432
  credentials =
    user = admin
    password = secret
```

### Not Yet Implemented ⚠️

These features appear in test suites but are not yet implemented:

#### Section Headers
```ccl
== Section 1 ==
key1 = value1

== Section 2 ==
key2 = value2
```

**Status**: Planned (high priority)
**Affected Tests**: 14 tests
**Impact**: Would enable cleaner configuration organization

#### Duplicate Key Lists
```ccl
item = first
item = second
item = third
# Expected: Creates list [first, second, third]
# Current: Last value wins (only "third")
```

**Status**: Planned (high priority)
**Affected Tests**: 6 tests
**Impact**: More intuitive list creation

---

## Value Interpretation

Sickle uses intelligent heuristics to interpret values:

### Rules

1. **Empty Values** → Empty string singletons
   ```ccl
   key =
   # Becomes: Singleton("")
   ```

2. **Plain Strings** → Singleton strings
   ```ccl
   name = Santa
   # Becomes: Singleton("Santa")
   ```

3. **Values with `=`** → Recursive parsing (if keys look valid)
   ```ccl
   database = host = localhost
   # Becomes: Map { "host" => Singleton("localhost") }
   ```

4. **Command-line Strings** → Treated as strings (not parsed)
   ```ccl
   args = --flag=value --another
   # Becomes: Singleton("--flag=value --another")
   # Reason: Keys start with '-' (invalid CCL keys)
   ```

5. **Multiple Values** → Currently: last wins; Planned: create list
   ```ccl
   item = first
   item = second
   # Current: Singleton("second")
   # Planned: List([Singleton("first"), Singleton("second")])
   ```

### Validation

Nested parsing validates that keys:
- Don't start with `-` (command-line flags)
- Don't contain spaces
- Look like valid CCL identifiers

---

## Error Handling

All errors are strongly typed using the `Error` enum:

| Error Type | Description | Example |
|------------|-------------|---------|
| `ParseError` | Failed to parse CCL syntax | Invalid syntax |
| `MissingKey` | Requested key not found | `get("nonexistent")` |
| `NotAMap` | Map operation on non-map | `get()` on singleton |
| `NotAList` | List operation on non-list | `as_list()` on singleton |
| `NotASingleton` | String operation on non-singleton | `as_str()` on map |
| `ValueError` | Type conversion failed | `parse_value::<u16>()` on "abc" |

### Error Handling Examples

```rust
use sickle::{parse, Error};

let model = parse("name = Santa")?;

// Handle specific errors
match model.get("age") {
    Ok(value) => println!("Age: {}", value.as_str()?),
    Err(Error::MissingKey(key)) => println!("Key '{}' not found", key),
    Err(e) => println!("Other error: {}", e),
}

// Type conversion errors
let result: Result<u16, _> = model.get("name")?.parse_value();
match result {
    Err(Error::ValueError(msg)) => println!("Parse error: {}", msg),
    _ => {}
}
```

---

## Cargo Features

Sickle supports optional features via Cargo feature flags:

### Available Features

| Feature | Default | Description | Dependencies |
|---------|---------|-------------|--------------|
| `serde` | ✅ Yes | Serde serialization/deserialization | `serde`, `serde_derive` |
| `intern` | ❌ No | String interning for memory efficiency | `string-interner` |

### Planned Features

| Feature | Status | Description |
|---------|--------|-------------|
| `section-headers` | Planned | Support `== Section ==` syntax |
| `duplicate-key-lists` | Planned | Auto-create lists from duplicate keys |
| `typed-access` | Planned | Convenience methods: `get_string()`, `get_int()` |
| `list-indexing` | Planned | Advanced list operations |

### Using Features

```toml
# Cargo.toml - Default configuration
[dependencies]
sickle = "0.1"

# Without serde
[dependencies]
sickle = { version = "0.1", default-features = false }

# With string interning
[dependencies]
sickle = { version = "0.1", features = ["intern"] }
```

---

## Test Coverage

Current test suite statistics (from CCL test data repository):

| Metric | Count |
|--------|-------|
| Total Test Suites | 13 |
| Total Tests | 327 |
| Passing Tests | 110 (34%) |
| Failing Tests | 217 (66%) |

### Suite Breakdown

| Suite | Tests | Pass | Fail | Status |
|-------|-------|------|------|--------|
| `api_core_ccl_parsing` | 8 | 8 | 0 | ✅ Fully Implemented |
| `api_errors` | 6 | 6 | 0 | ✅ Fully Implemented |
| `api_edge_cases` | 35 | 21 | 14 | ⚠️ Partial |
| `api_proposed_behavior` | 60 | 14 | 46 | ⚠️ Many features planned |
| `api_typed_access` | 74 | 25 | 49 | ⚠️ Partial |
| `api_list_access` | 21 | 4 | 17 | ⚠️ Partial |
| Others | 123 | 32 | 91 | ⚠️ Various |

### Test Execution

Run the comprehensive test suite:

```bash
# Run all CCL test suites
just test-ccl

# Or directly with cargo
cargo test -p sickle test_all_ccl_suites_comprehensive -- --nocapture
```

---

## Roadmap

Planned features in priority order:

### High Priority

1. **Section Headers** (`== Section ==`)
   - Affected tests: 14
   - Effort: Medium
   - Impact: Better configuration organization

2. **Duplicate Key Lists**
   - Affected tests: 6
   - Effort: Medium
   - Impact: More intuitive API

### Medium Priority

3. **String Interning** (memory optimization)
   - Affected tests: 0
   - Effort: Low
   - Impact: Reduced memory for large configs

### Low Priority

4. **Typed Access Helpers** (`get_string()`, `get_int()`, `get_bool()`)
   - Affected tests: 49
   - Effort: Low
   - Impact: Convenience methods

5. **Advanced List Operations** (indexing, iteration)
   - Affected tests: 17
   - Effort: Medium
   - Impact: Better list manipulation

---

## Contributing

When adding new features:

1. Update `CCL_FEATURES.toml` with feature details
2. Add entry to this document's relevant section
3. Update test coverage statistics
4. Add examples to `examples/` directory
5. Consider adding Cargo feature flag if optional

---

## References

- [CCL Specification](https://ccl.tylerbutler.com)
- [CCL Test Data Repository](https://github.com/tylerbutler/ccl-test-data)
- [API Documentation](https://docs.rs/sickle)

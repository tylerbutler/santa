# Critical Improvements Plan for Santa

## Priority 1: Error Handling (High Impact)

### 1.1 Replace `.unwrap()` with proper error handling
**Files affected:** `src/main.rs`, `src/configuration.rs`, `src/data.rs`, `src/sources.rs`

**Changes:**
- Replace `.unwrap()` calls with `?` operator and proper error propagation
- Add `anyhow::Context` for meaningful error messages
- Create custom error types for domain-specific errors

**Example transformations:**
```rust
// Before: configuration.rs:34
let data: SantaConfig = serde_yaml::from_str(yaml_str).unwrap();

// After:
let data: SantaConfig = serde_yaml::from_str(yaml_str)
    .with_context(|| format!("Failed to parse config from: {}", yaml_str))?;
```

**Estimated effort:** 2-3 hours

### 1.2 Remove production `todo!()` calls
**Files affected:** `src/configuration.rs:83-84`, `src/data.rs:111`, `src/main.rs:145`

**Changes:**
- Replace `todo!()` in `configuration.rs:83` with proper error handling
- Implement missing functionality or return appropriate errors
- Add comprehensive test coverage for these code paths

**Estimated effort:** 1-2 hours

### 1.3 Add result types to main functions
**Files affected:** `src/main.rs`, `src/commands.rs`

**Changes:**
- Convert command functions to return `Result<(), anyhow::Error>`
- Propagate errors properly through the call stack
- Add structured error reporting in `main()`

**Estimated effort:** 1 hour

## Priority 2: Dependency Management (Medium Impact)

### 2.1 Update Cargo.toml dependencies
**Files affected:** `Cargo.toml`

**Changes:**
- Update to latest compatible versions:
  - `clap = "4.4"` 
  - `serde_yaml = "0.9"`
  - `config = "0.14"`
  - `anyhow = "1.0.75"`
- Remove commented-out dependencies
- Add `rust-version = "1.70.0"` for modern features

**Estimated effort:** 30 minutes

### 2.2 Clean up unused dependencies
**Files affected:** `Cargo.toml`, source files

**Changes:**
- Run `cargo machete` or similar to identify unused deps
- Remove commented-out extern crate declarations
- Remove `#![allow(unused)]` from main.rs after cleanup

**Estimated effort:** 30 minutes

## Priority 3: Memory Management (Medium Impact)

### 3.1 Reduce unnecessary clones
**Files affected:** `src/commands.rs`, `src/configuration.rs`

**Changes:**
- Replace `config.clone().groups(data)` with borrowed access
- Use `&str` instead of `String` where possible
- Implement `Clone` only where necessary

**Example transformations:**
```rust
// Before: commands.rs:20
.filter(|source| config.clone().source_is_enabled(source))

// After:
.filter(|source| config.source_is_enabled(source))
```

**Estimated effort:** 1-2 hours

### 3.2 Fix ownership patterns
**Files affected:** `src/configuration.rs:51`, `src/sources.rs`

**Changes:**
- Change `source_is_enabled(self, source: &PackageSource)` to take `&self`
- Review all methods taking `self` unnecessarily
- Use borrowing patterns consistently

**Estimated effort:** 1 hour

## Implementation Order

1. **Week 1:** Error handling improvements (1.1, 1.2, 1.3)
2. **Week 1:** Dependency updates (2.1, 2.2)  
3. **Week 2:** Memory management fixes (3.1, 3.2)

## Success Criteria

- [ ] Zero `unwrap()` calls in production code paths
- [ ] Zero `todo!()` macros in production code
- [ ] All dependencies updated to latest compatible versions
- [ ] Reduced memory allocations in hot paths
- [ ] All tests passing after changes
- [ ] No clippy warnings on pedantic lint level

## Risk Assessment

**Low Risk:**
- Dependency updates (can be rolled back easily)
- Memory optimization (maintains same API)

**Medium Risk:**
- Error handling changes (affects API contracts)
- Requires comprehensive testing

**Mitigation:**
- Implement changes incrementally
- Add integration tests before refactoring
- Use feature flags for major changes
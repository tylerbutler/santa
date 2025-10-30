# Santa API Documentation

**Developer guide for using Santa as a library**

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Core Concepts](#core-concepts)
4. [Configuration API](#configuration-api)
5. [Package Sources](#package-sources)
6. [Script Generation](#script-generation)
7. [Caching](#caching)
8. [Error Handling](#error-handling)
9. [Advanced Usage](#advanced-usage)
10. [Examples](#examples)

---

## Overview

Santa can be used as a Rust library to integrate package management capabilities into your own applications. The library provides:

- **Configuration management** with CCL/YAML support
- **Multi-source package operations** across different package managers
- **Safe script generation** with template-based rendering
- **High-performance caching** with TTL and LRU eviction
- **Async operations** using Tokio
- **Comprehensive error handling** with context

### Add to Your Project

```toml
[dependencies]
santa = { git = "https://github.com/tylerbutler/santa" }
tokio = { version = "1.47", features = ["full"] }
```

---

## Getting Started

### Basic Usage

```rust
use santa::{SantaConfig, SantaData, sources::PackageCache};
use santa::commands::status_command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let mut config = SantaConfig::default();

    // Load package data
    let data = SantaData::default();

    // Create cache
    let cache = PackageCache::new();

    // Check package status
    status_command(&mut config, &data, cache, &false).await?;

    Ok(())
}
```

### Library Initialization

```rust
use santa::{SantaConfig, SantaData};
use std::path::Path;

// Load from file
let config = SantaConfig::load_from(Path::new("config.ccl"))?;

// Use built-in configuration
let data = SantaData::default();

// Get enabled sources
let sources = data.sources(&config);
```

---

## Core Concepts

### SantaConfig

Central configuration structure managing sources, packages, and settings.

```rust
use santa::configuration::SantaConfig;

// Create default config
let mut config = SantaConfig::default();

// Load from file
let config = SantaConfig::load_from(Path::new("santa.ccl"))?;

// Check if source is enabled
if config.source_is_enabled(&source) {
    println!("Source {} is enabled", source.name_str());
}

// Get package groups by source
let groups = config.groups(&data);
for (source_name, packages) in groups {
    println!("{}: {:?}", source_name, packages);
}
```

### SantaData

Contains built-in package and source definitions.

```rust
use santa::data::SantaData;

// Load default data
let data = SantaData::default();

// Get all sources
let all_sources = &data.sources;

// Get enabled sources based on config
let enabled = data.sources(&config);

// Export to CCL format
let ccl_output = data.export();
println!("{}", ccl_output);
```

### PackageSource

Represents individual package managers.

```rust
use santa::sources::PackageSource;
use santa::data::KnownSources;

// Access specific source
let brew = PackageSource::new(
    "brew",
    "Homebrew",
    "brew",
    "install",
    "list",
    KnownSources::Brew
)?;

// Check if available on system
if brew.is_available() {
    println!("Homebrew is installed");
}

// Get manager name
println!("Manager: {}", brew.manager());
```

---

## Configuration API

### Loading Configuration

```rust
use santa::configuration::SantaConfig;
use std::path::Path;

// From file
let config = SantaConfig::load_from(Path::new("config.ccl"))?;

// From default location (~/.config/santa/config.yaml)
let config = SantaConfig::from_file()?;

// Using builder pattern
use santa::configuration::SantaConfigBuilder;

let config = SantaConfigBuilder::default()
    .sources(vec!["brew".into(), "cargo".into()])
    .packages(vec!["git".into(), "rust".into()])
    .build()?;
```

### Configuration Validation

```rust
use validator::Validate;

// Validate configuration
match config.validate() {
    Ok(_) => println!("Config is valid"),
    Err(e) => eprintln!("Validation errors: {}", e),
}
```

### Environment Override

```rust
use santa::configuration::env::apply_env_overrides;

// Apply environment variable overrides
apply_env_overrides(&mut config)?;

// Specific environment variables:
// SANTA_SOURCES="brew,cargo"
// SANTA_PACKAGES="git,rust"
// SANTA_BUILTIN_ONLY="true"
```

### Hot-Reloading

```rust
use santa::configuration::watcher::ConfigWatcher;
use std::sync::Arc;
use tokio::sync::RwLock;

let config = Arc::new(RwLock::new(SantaConfig::default()));
let config_clone = Arc::clone(&config);

// Start watching for changes
let watcher = ConfigWatcher::new(
    Path::new("config.ccl"),
    config_clone
)?;

// Config updates automatically when file changes
tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
let current = config.read().await;
```

---

## Package Sources

### Working with Sources

```rust
use santa::data::{SantaData, SourceList};
use santa::traits::PackageManager;

let data = SantaData::default();
let config = SantaConfig::default();

// Get enabled sources
let sources: SourceList = data
    .sources
    .iter()
    .filter(|s| config.source_is_enabled(s))
    .cloned()
    .collect();

// Check package availability
for source in sources.iter() {
    if source.is_available() {
        let packages = source.list_installed().await?;
        println!("{}: {} packages", source.name_str(), packages.len());
    }
}
```

### Custom Package Sources

```rust
use santa::sources::PackageSource;
use santa::data::KnownSources;

// Define custom source
let custom = PackageSource::new(
    "custom",
    "Custom Package Manager",
    "custom-pm",
    "install",
    "list",
    KnownSources::Other
)?;

// Use with standard operations
if custom.is_available() {
    let installed = custom.list_installed().await?;
    println!("Installed: {:?}", installed);
}
```

### Async Operations

```rust
use futures::future::try_join_all;

// Concurrent package list fetching
let tasks: Vec<_> = sources
    .iter()
    .map(|source| source.list_installed())
    .collect();

let results = try_join_all(tasks).await?;
for (source, packages) in sources.iter().zip(results) {
    println!("{}: {} packages", source.name_str(), packages.len());
}
```

---

## Script Generation

### Basic Script Generation

```rust
use santa::script_generator::{ScriptGenerator, ScriptFormat, ExecutionMode};

// Create generator
let generator = ScriptGenerator::new()?;

// Generate shell script
let packages = vec!["git".to_string(), "rust".to_string()];
let script = generator.generate_install_script(
    &packages,
    "brew",
    ScriptFormat::Shell,
    "homebrew"
)?;

// Write to file
std::fs::write("install.sh", script)?;
```

### Platform Detection

```rust
use santa::script_generator::ScriptFormat;

// Auto-detect format
let format = ScriptFormat::auto_detect();

// Get extension
let ext = format.extension(); // "sh", "ps1", or "bat"

// Get template name
let template = format.install_template_name();
```

### Custom Templates

```rust
use santa::script_generator::ScriptGenerator;
use tera::{Tera, Context};

let mut generator = ScriptGenerator::new()?;

// Access internal Tera engine (advanced)
// Add custom template
let template = r#"
#!/bin/bash
{% for pkg in packages %}
echo "Installing {{ pkg }}"
{{ manager }} install {{ pkg | shell_escape }}
{% endfor %}
"#;

// Generate with custom template
let mut context = Context::new();
context.insert("packages", &vec!["git", "rust"]);
context.insert("manager", "brew");
```

### Execution Modes

```rust
use santa::script_generator::ExecutionMode;

// Safe mode (default): generates scripts only
let mode = ExecutionMode::Safe;

// Direct execution: runs commands immediately
let mode = ExecutionMode::Execute;

// Check mode
if mode == ExecutionMode::Safe {
    println!("Safe mode: scripts will be generated");
}
```

---

## Caching

### Package Cache

```rust
use santa::sources::PackageCache;
use std::time::Duration;

// Default cache (5 min TTL, 1000 entries)
let cache = PackageCache::new();

// Custom configuration
let cache = PackageCache::with_config(
    Duration::from_secs(600),  // 10 min TTL
    2000                        // 2000 max entries
);

// Cache operations
let packages = vec!["git".to_string(), "rust".to_string()];
cache.insert("brew".to_string(), packages.clone());

// Retrieve from cache
if let Some(cached) = cache.get("brew") {
    println!("Cached: {:?}", cached);
}

// Invalidate specific entry
cache.invalidate("brew");

// Clear all
cache.clear();
```

### Cache Statistics

```rust
use santa::sources::PackageCache;

let cache = PackageCache::new();
let stats = cache.stats();

println!("Cache entries: {}", stats.entries);
println!("Weighted size: {}", stats.weighted_size);
```

### Async Caching

```rust
use santa::traits::Cacheable;

let cache = PackageCache::new();
let source = PackageSource::new(/* ... */)?;

// Async cache population
cache.cache_for_async(&source).await?;

// Check if cached
if let Some(packages) = cache.get_packages(&source) {
    println!("Using cached data");
}
```

---

## Error Handling

### Error Types

```rust
use santa::errors::{SantaError, Result};

fn example() -> Result<()> {
    // Configuration errors
    let config = SantaConfig::load_from(path)
        .map_err(SantaError::Config)?;

    // Package source errors
    if !source.is_available() {
        return Err(SantaError::PackageSource(
            format!("{} is not available", source.name_str())
        ));
    }

    // Security errors
    if !validate_package_name(&name) {
        return Err(SantaError::Security(
            "Invalid package name".to_string()
        ));
    }

    Ok(())
}
```

### Error Context

```rust
use anyhow::Context;

// Add context to errors
let config = SantaConfig::load_from(path)
    .context("Failed to load configuration")?;

let packages = source.list_installed().await
    .context(format!("Failed to list packages from {}", source.name_str()))?;
```

### Error Patterns

```rust
match operation().await {
    Ok(result) => println!("Success: {:?}", result),
    Err(SantaError::PackageSource(msg)) => {
        eprintln!("Package source error: {}", msg);
    }
    Err(SantaError::Security(msg)) => {
        eprintln!("Security violation: {}", msg);
        std::process::exit(1);
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

---

## Advanced Usage

### Structured Concurrency

```rust
use futures::future::try_join_all;
use std::sync::Arc;
use tokio::sync::RwLock;

// Concurrent caching with shared state
let cache = Arc::new(RwLock::new(PackageCache::new()));

let cache_tasks: Vec<_> = sources
    .iter()
    .map(|source| {
        let cache_clone = Arc::clone(&cache);
        let source = source.clone();
        async move {
            let cache = cache_clone.write().await;
            cache.cache_for_async(&source).await
        }
    })
    .collect();

// All tasks structured and awaited together
try_join_all(cache_tasks).await?;

// Extract cache
let cache = Arc::try_unwrap(cache)
    .map_err(|_| SantaError::Concurrency("Cache still in use".into()))?
    .into_inner();
```

### Custom Traits

```rust
use santa::traits::{PackageManager, Cacheable};

// Implement custom package manager
struct MyPackageManager {
    name: String,
}

impl PackageManager for MyPackageManager {
    fn name(&self) -> &str {
        &self.name
    }

    async fn list_installed(&self) -> Result<Vec<String>> {
        // Custom implementation
        Ok(vec![])
    }

    async fn install(&self, packages: &[String]) -> Result<()> {
        // Custom implementation
        Ok(())
    }
}
```

### Plugin System

```rust
use santa::plugins::{Plugin, PluginManager};

// Define custom plugin
struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn on_pre_install(&self, packages: &[String]) -> Result<()> {
        println!("Pre-install hook: {:?}", packages);
        Ok(())
    }
}

// Register plugin
let mut manager = PluginManager::new();
manager.register(Box::new(MyPlugin))?;
```

---

## Examples

### Complete Application

```rust
use santa::{SantaConfig, SantaData, sources::PackageCache};
use santa::commands::{status_command, install_command};
use santa::script_generator::{ExecutionMode, ScriptFormat};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize
    let mut config = SantaConfig::default();
    let data = SantaData::default();
    let cache = PackageCache::new();

    // Show status
    println!("Package Status:");
    status_command(&mut config, &data, cache.clone(), &false).await?;

    // Generate installation scripts
    println!("\nGenerating installation scripts...");
    install_command(
        &mut config,
        &data,
        cache,
        ExecutionMode::Safe,
        ScriptFormat::auto_detect(),
        Path::new("./scripts")
    ).await?;

    println!("Scripts generated successfully!");
    Ok(())
}
```

### Configuration Builder

```rust
use santa::configuration::{SantaConfig, SantaConfigBuilder};

fn create_custom_config() -> Result<SantaConfig, Box<dyn std::error::Error>> {
    let config = SantaConfigBuilder::default()
        .sources(vec![
            "brew".into(),
            "cargo".into(),
            "apt".into(),
        ])
        .packages(vec![
            "git".into(),
            "rust".into(),
            "ripgrep".into(),
            "fd".into(),
        ])
        .build()?;

    Ok(config)
}
```

### Custom Source Integration

```rust
use santa::sources::PackageSource;
use santa::data::KnownSources;
use santa::traits::PackageManager;

async fn integrate_custom_source() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom source
    let nixpkgs = PackageSource::new(
        "nix",
        "Nix Package Manager",
        "nix-env",
        "-iA nixpkgs.",
        "-q",
        KnownSources::Nix
    )?;

    // Check availability
    if !nixpkgs.is_available() {
        eprintln!("Nix is not installed");
        return Ok(());
    }

    // List installed packages
    let installed = nixpkgs.list_installed().await?;
    println!("Nix packages installed: {}", installed.len());

    Ok(())
}
```

### Performance Monitoring

```rust
use santa::sources::PackageCache;
use std::time::Instant;

async fn monitor_performance() -> Result<(), Box<dyn std::error::Error>> {
    let cache = PackageCache::new();

    // Measure cache performance
    let start = Instant::now();

    // First call (cache miss)
    let packages1 = fetch_packages(&cache).await?;
    let duration1 = start.elapsed();

    // Second call (cache hit)
    let start2 = Instant::now();
    let packages2 = fetch_packages(&cache).await?;
    let duration2 = start2.elapsed();

    println!("First call: {:?}", duration1);
    println!("Second call (cached): {:?}", duration2);
    println!("Speedup: {:.2}x", duration1.as_secs_f64() / duration2.as_secs_f64());

    Ok(())
}
```

---

## Further Reading

- [User Guide](./USER_GUIDE.md) - Complete user documentation
- [Architecture Guide](./ARCHITECTURE.md) - Internal design details
- [Contributing](../CONTRIBUTING.md) - Development guidelines
- [Rust API Docs](https://docs.rs/santa) - Generated API documentation

---

**Need help?** [Open an issue](https://github.com/tylerbutler/santa/issues) or check the [discussions](https://github.com/tylerbutler/santa/discussions).

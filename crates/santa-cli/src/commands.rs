//! Command implementations for Santa CLI operations.
//!
//! This module contains the core business logic for all Santa commands,
//! including status checking, package installation, and configuration management.
//!
//! # Commands
//!
//! - [`status_command`]: Display package availability status across sources
//! - [`install_command`]: Install packages using script generation or direct execution
//! - [`config_command`]: Display current configuration
//!
//! # Architecture
//!
//! All commands follow a consistent pattern:
//! 1. Load and validate configuration
//! 2. Filter enabled package sources
//! 3. Perform async operations with proper error handling
//! 4. Use structured concurrency for parallel operations
//!
//! # Examples
//!
//! ```rust,no_run
//! use santa::{SantaConfig, SantaData, sources::PackageCache};
//! use santa::configuration::SantaConfigExt;
//! use santa::commands::status_command;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = SantaConfig::default_for_platform();
//! let data = SantaData::default();
//! let cache = PackageCache::new();
//!
//! // Display package status
//! status_command(&mut config, &data, cache, &false).await?;
//! # Ok(())
//! # }
//! ```

use crate::configuration::SantaConfigExt; // Import extension trait for method access
use crate::configuration::UnknownPackageReason;
use crate::data::SantaData;
use crate::data::SourceList;
use crate::errors::{Result, SantaError};
use crate::script_generator::{ExecutionMode, ScriptFormat};
use crate::{configuration::SantaConfig, sources::PackageCache};
use futures::future::try_join_all;
use std::sync::Arc;
use tabular::{Row, Table};
use tokio::sync::RwLock;

use tracing::debug;

#[cfg(test)]
mod tests;

/// Display the status of all configured packages across enabled sources.
///
/// This command performs the following operations:
/// 1. Filters sources to only those enabled in configuration
/// 2. Concurrently caches package data from all sources
/// 3. Displays a formatted table showing package availability
///
/// # Arguments
///
/// * `config` - Mutable reference to Santa configuration
/// * `data` - Reference to Santa data containing source definitions
/// * `cache` - Package cache for performance optimization
/// * `all` - If true, show all packages; if false, only show missing packages
/// * `installed` - If true, show only installed packages
/// * `missing` - If true, show only missing packages
/// * `source_filter` - Optional source name to filter by
///
/// # Returns
///
/// Returns `Ok(())` on success, or a [`SantaError`] if operations fail.
///
/// # Examples
///
/// ```rust,no_run
/// use santa::{SantaConfig, SantaData, sources::PackageCache};
/// use santa::configuration::SantaConfigExt;
/// use santa::commands::status_command;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut config = SantaConfig::default_for_platform();
/// let data = SantaData::default();
/// let cache = PackageCache::new();
///
/// // Show only missing packages
/// status_command(&mut config, &data, cache, &false, &false, &true, None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn status_command(
    config: &mut SantaConfig,
    data: &SantaData,
    cache: PackageCache,
    all: &bool,
    installed: &bool,
    missing: &bool,
    source_filter: Option<&str>,
) -> Result<()> {
    use std::collections::HashMap;
    use std::time::Instant;

    #[cfg(debug_assertions)]
    let start = Instant::now();

    // filter sources to those enabled in the config (avoiding clone)
    #[cfg(debug_assertions)]
    let filter_start = Instant::now();
    let mut sources: SourceList = data
        .sources
        .iter()
        .filter(|source| config.source_is_enabled(source))
        .cloned()
        .collect();

    // Apply source filter if provided
    if let Some(source_name) = source_filter {
        sources.retain(|s| s.name_str() == source_name);
        if sources.is_empty() {
            return Err(SantaError::Config(anyhow::anyhow!(
                "Source '{}' not found or not enabled",
                source_name
            )));
        }
    }

    #[cfg(debug_assertions)]
    debug!("⏱️  Source filtering took: {:?}", filter_start.elapsed());

    // Show user what's being checked
    let source_names: Vec<String> = sources.iter().map(|s| s.name().to_string()).collect();
    eprintln!("Checking package managers: {}...", source_names.join(", "));

    // Track durations for each source
    let durations = Arc::new(RwLock::new(HashMap::new()));

    // Use structured concurrency to cache data for all sources concurrently
    #[cfg(debug_assertions)]
    let cache_setup_start = Instant::now();
    let cache = Arc::new(RwLock::new(cache));
    let cache_tasks: Vec<_> = sources
        .iter()
        .map(|source| {
            let cache_clone: Arc<RwLock<PackageCache>> = Arc::clone(&cache);
            let durations_clone = Arc::clone(&durations);
            let source = source.clone();
            async move {
                let task_start = Instant::now();
                eprint!("  Checking {}... ", source.name());
                let cache = cache_clone.write().await;
                let result = cache.cache_for_async(&source).await;
                let duration = task_start.elapsed();

                // Store duration
                let mut durations = durations_clone.write().await;
                durations.insert(source.name_str(), duration);

                eprintln!("✓");
                #[cfg(debug_assertions)]
                debug!("⏱️  Cache for {} took: {:?}", source.name(), duration);
                result
            }
        })
        .collect();
    #[cfg(debug_assertions)]
    debug!("⏱️  Cache setup took: {:?}", cache_setup_start.elapsed());

    // All tasks are structured under this scope - they'll be awaited together
    #[cfg(debug_assertions)]
    let caching_start = Instant::now();
    match try_join_all(cache_tasks).await {
        Ok(_) => debug!("Successfully cached data for all sources"),
        Err(e) => tracing::error!("Some cache operations failed: {}", e),
    }
    #[cfg(debug_assertions)]
    debug!("⏱️  Total caching took: {:?}", caching_start.elapsed());
    eprintln!();

    // Extract durations from Arc
    let durations = Arc::try_unwrap(durations)
        .map_err(|_| {
            SantaError::Concurrency("Failed to unwrap durations - still in use".to_string())
        })?
        .into_inner();

    // Extract cache from Arc<Mutex<>> for further use
    #[cfg(debug_assertions)]
    let unwrap_start = Instant::now();
    let cache = Arc::try_unwrap(cache)
        .map_err(|_| SantaError::Concurrency("Failed to unwrap cache - still in use".to_string()))?
        .into_inner();
    #[cfg(debug_assertions)]
    debug!("⏱️  Cache unwrap took: {:?}", unwrap_start.elapsed());

    #[cfg(debug_assertions)]
    let display_start = Instant::now();
    for source in &sources {
        #[cfg(debug_assertions)]
        let groups_start = Instant::now();
        let groups = config.groups(data);
        #[cfg(debug_assertions)]
        debug!(
            "⏱️  Groups computation for {} took: {:?}",
            source.name(),
            groups_start.elapsed()
        );

        for (key, pkgs) in groups {
            if source.name() == &key {
                #[cfg(debug_assertions)]
                let table_start = Instant::now();
                let pkg_count = pkgs.len();

                // Filter packages based on installed/missing flags
                let filtered_pkgs: Vec<String> = if *installed {
                    // Show only installed packages
                    pkgs.iter()
                        .filter(|p| cache.check(source, p, data))
                        .cloned()
                        .collect()
                } else if *missing {
                    // Show only missing packages (default behavior)
                    pkgs.iter()
                        .filter(|p| !cache.check(source, p, data))
                        .cloned()
                        .collect()
                } else {
                    // No filter or --all flag
                    pkgs.clone()
                };

                // Determine if we should show installed packages in table
                let include_installed = *all || *installed;
                let table = format!(
                    "{}",
                    source.table(&filtered_pkgs, &cache, data, include_installed)
                );

                #[cfg(debug_assertions)]
                debug!(
                    "⏱️  Table generation for {} ({} pkgs) took: {:?}",
                    source.name(),
                    pkg_count,
                    table_start.elapsed()
                );

                // Get duration for this source
                let duration_str = if let Some(duration) = durations.get(&source.name_str()) {
                    if duration.as_secs() > 0 {
                        format!(" - checked in {:.1}s", duration.as_secs_f64())
                    } else {
                        format!(" - checked in {}ms", duration.as_millis())
                    }
                } else {
                    String::new()
                };

                println!("{source} ({pkg_count} packages total{duration_str})");
                println!("{table}");
                break;
            }
        }
    }

    // Display unknown packages (no definition or no matching source)
    let unknown = config.unknown_packages(data);
    if !unknown.is_empty() {
        println!("Unknown ({} packages)", unknown.len());
        let mut table = Table::new("{:<} {:<} {:<}");
        for (pkg, reason) in &unknown {
            let (emoji, reason_str) = match reason {
                UnknownPackageReason::NoDefinition => ("👻", "no definition".to_string()),
                UnknownPackageReason::NoMatchingSource(available) => {
                    let sources: Vec<String> = available.iter().map(|s| s.to_string()).collect();
                    ("🚫", format!("available in: {}", sources.join(", ")))
                }
            };
            table.add_row(
                Row::new()
                    .with_cell(emoji)
                    .with_cell(pkg)
                    .with_cell(reason_str),
            );
        }
        println!("{table}");
    }

    #[cfg(debug_assertions)]
    debug!("⏱️  Display phase took: {:?}", display_start.elapsed());
    #[cfg(debug_assertions)]
    debug!("⏱️  TOTAL status_command took: {:?}", start.elapsed());

    Ok(())
}

/// Display the current Santa configuration.
///
/// Shows configuration in various formats depending on flags:
/// - Default: User configuration
/// - `packages=true`: Package definitions
/// - `builtin=true`: Built-in source definitions
///
/// # Arguments
///
/// * `config` - Reference to current Santa configuration
/// * `data` - Reference to Santa data with built-in definitions
/// * `packages` - Show package definitions instead of config
/// * `builtin` - Show only built-in configuration
///
/// # Returns
///
/// Returns `Ok(())` on success.
///
/// # Examples
///
/// ```rust,no_run
/// use santa::{SantaConfig, SantaData};
/// use santa::configuration::SantaConfigExt;
/// use santa::commands::config_command;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SantaConfig::default_for_platform();
/// let data = SantaData::default();
///
/// // Show current configuration
/// config_command(&config, &data, false, false)?;
/// # Ok(())
/// # }
/// ```
pub fn config_command(
    config: &SantaConfig,
    data: &SantaData,
    packages: bool,
    builtin: bool,
) -> Result<()> {
    if !builtin {
        println!("{}", config.export());
    } else if packages {
        println!("{:#?}", data);
    } else {
        println!("{:#?}", data.sources)
    }
    Ok(())
}

/// Install packages using safe script generation or direct execution.
///
/// This command generates platform-specific installation scripts or directly
/// executes package manager commands, depending on the execution mode.
///
/// # Safety
///
/// By default, this command operates in safe mode (script generation only).
/// Direct execution mode must be explicitly enabled and requires user confirmation.
///
/// # Arguments
///
/// * `config` - Mutable reference to Santa configuration
/// * `data` - Reference to Santa data containing source definitions
/// * `cache` - Package cache for performance
/// * `execution_mode` - Safe (script generation) or Execute (direct execution)
/// * `script_format` - Target script format (Shell, PowerShell, Batch)
/// * `output_dir` - Directory for generated scripts
///
/// # Returns
///
/// Returns `Ok(())` on success, or a [`SantaError`] on failure.
///
/// # Examples
///
/// ```rust,no_run
/// use santa::{SantaConfig, SantaData, sources::PackageCache};
/// use santa::configuration::SantaConfigExt;
/// use santa::script_generator::{ExecutionMode, ScriptFormat};
/// use santa::commands::install_command;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut config = SantaConfig::default_for_platform();
/// let data = SantaData::default();
/// let cache = PackageCache::new();
///
/// // Generate installation scripts (safe mode)
/// install_command(
///     &mut config,
///     &data,
///     cache,
///     ExecutionMode::Safe,
///     ScriptFormat::auto_detect(),
///     Path::new("./scripts")
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn install_command(
    config: &mut SantaConfig,
    data: &SantaData,
    cache: PackageCache,
    execution_mode: ExecutionMode,
    script_format: ScriptFormat,
    output_dir: &std::path::Path,
) -> Result<()> {
    // let config = config.clone();
    // filter sources to those enabled in the config (avoiding clone)
    let sources: SourceList = data
        .sources
        .iter()
        .filter(|source| config.source_is_enabled(source))
        .cloned()
        .collect();

    // for (k, v) in config.groups(&data) {
    //     error!("{} {:?}", k, v);
    // }

    // Use structured concurrency to cache data for all sources concurrently
    let cache = Arc::new(RwLock::new(cache));
    let cache_tasks: Vec<_> = sources
        .iter()
        .map(|source| {
            let cache_clone: Arc<RwLock<PackageCache>> = Arc::clone(&cache);
            let source = source.clone();
            async move {
                debug!("Async stats for {}", source.name());
                let cache = cache_clone.write().await;
                cache.cache_for_async(&source).await
            }
        })
        .collect();

    // All caching tasks are structured under this scope
    match try_join_all(cache_tasks).await {
        Ok(_) => debug!("Successfully cached data for install operation"),
        Err(e) => tracing::error!("Some install cache operations failed: {}", e),
    }

    // Extract cache from Arc<Mutex<>> for further use
    let cache = Arc::try_unwrap(cache)
        .map_err(|_| {
            SantaError::Concurrency("Failed to unwrap install cache - still in use".to_string())
        })?
        .into_inner();

    // let config = config.clone();
    for source in &sources {
        let groups = config.groups(data);
        for (key, pkgs) in groups {
            if source.name() == &key {
                let pkgs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| !cache.check(source, p, data))
                    .map(|p| p.to_string())
                    .collect();
                source.exec_install(
                    config,
                    data,
                    pkgs,
                    execution_mode.clone(),
                    script_format.clone(),
                    output_dir,
                )?;
            }
        }
    }
    Ok(())
}

/// Add packages to the Santa configuration.
///
/// This command adds one or more packages to the configuration file,
/// validating that they exist in the package database.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `package_names` - List of package names to add
/// * `data` - Reference to Santa data containing package definitions
///
/// # Returns
///
/// Returns `Ok(())` on success, or a [`SantaError`] on failure.
pub async fn add_command(
    config_path: &std::path::Path,
    package_names: Vec<String>,
    data: &SantaData,
) -> Result<()> {
    // Load current config
    let mut config = SantaConfig::load_from(config_path)?;

    // Validate packages exist in database
    for pkg in &package_names {
        if !data.packages.contains_key(pkg) {
            return Err(SantaError::Config(anyhow::anyhow!(
                "Package '{}' not found in database",
                pkg
            )));
        }
    }

    // Add packages to config (avoiding duplicates)
    for pkg in package_names {
        if !config.packages.contains(&pkg) {
            config.packages.push(pkg.clone());
            println!("Added package: {}", pkg);
        } else {
            println!("Package already in config: {}", pkg);
        }
    }

    // Save config back to CCL format
    let ccl_content = sickle::to_string(&config)
        .map_err(|e| SantaError::Config(anyhow::anyhow!("Failed to serialize config: {}", e)))?;
    std::fs::write(config_path, ccl_content).map_err(SantaError::Io)?;

    println!("\nConfiguration updated: {}", config_path.display());
    Ok(())
}

/// Remove packages from the Santa configuration.
///
/// This command removes one or more packages from the configuration file.
/// Optionally uninstalls the packages before removing them.
///
/// # Arguments
///
/// * `config_path` - Path to the configuration file
/// * `package_names` - List of package names to remove
/// * `uninstall` - If true, uninstall packages before removing from config
///
/// # Returns
///
/// Returns `Ok(())` on success, or a [`SantaError`] on failure.
pub async fn remove_command(
    config_path: &std::path::Path,
    package_names: Vec<String>,
    uninstall: bool,
) -> Result<()> {
    // Load current config
    let mut config = SantaConfig::load_from(config_path)?;

    // If uninstall requested, handle that first
    if uninstall {
        println!("Uninstall functionality not yet implemented");
        // TODO: Implement uninstall logic
    }

    // Remove packages from config
    let original_count = config.packages.len();
    config.packages.retain(|pkg| !package_names.contains(pkg));
    let removed_count = original_count - config.packages.len();

    if removed_count == 0 {
        println!("No packages were removed (not found in config)");
        return Ok(());
    }

    // Save config back to CCL format
    let ccl_content = sickle::to_string(&config)
        .map_err(|e| SantaError::Config(anyhow::anyhow!("Failed to serialize config: {}", e)))?;
    std::fs::write(config_path, ccl_content).map_err(SantaError::Io)?;

    println!("\nRemoved {} package(s) from configuration", removed_count);
    println!("Configuration updated: {}", config_path.display());
    Ok(())
}

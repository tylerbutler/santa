use crate::configuration::{SantaConfig, SantaConfigExt};
use crate::data::KnownSources;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tracing::{debug, info, warn};

/// Environment variable configuration support for Santa
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// Prefix for Santa environment variables
    pub prefix: String,
    /// Map of environment variable names to their values
    pub variables: HashMap<String, String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            prefix: "SANTA_".to_string(),
            variables: HashMap::new(),
        }
    }
}

/// Configuration that can be loaded from environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfigOverrides {
    /// Override log level via SANTA_LOG_LEVEL
    pub log_level: Option<String>,
    /// Override config file path via SANTA_CONFIG_PATH
    pub config_path: Option<String>,
    /// Override sources via SANTA_SOURCES (comma-separated)
    pub sources: Option<String>,
    /// Override packages via SANTA_PACKAGES (comma-separated)
    pub packages: Option<String>,
    /// Enable builtin-only mode via SANTA_BUILTIN_ONLY
    pub builtin_only: Option<bool>,
    /// Override cache TTL via SANTA_CACHE_TTL_SECONDS
    pub cache_ttl_seconds: Option<u64>,
    /// Override cache size via SANTA_CACHE_SIZE
    pub cache_size: Option<u64>,
    /// Enable verbose logging via SANTA_VERBOSE
    pub verbose: Option<u8>,
    /// Custom data directory via SANTA_DATA_DIR
    pub data_dir: Option<String>,
    /// Enable hot-reloading via SANTA_HOT_RELOAD
    pub hot_reload: Option<bool>,
}

impl EnvironmentConfig {
    /// Create new environment configuration with custom prefix
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            variables: HashMap::new(),
        }
    }

    /// Load all Santa environment variables
    pub fn load_from_env(&mut self) -> Result<()> {
        info!(
            "Loading configuration from environment variables with prefix: {}",
            self.prefix
        );

        for (key, value) in env::vars() {
            if key.starts_with(&self.prefix) {
                debug!("Found Santa environment variable: {}={}", key, value);
                self.variables.insert(key, value);
            }
        }

        info!("Loaded {} environment variables", self.variables.len());
        Ok(())
    }

    /// Get environment variable value
    pub fn get(&self, key: &str) -> Option<&String> {
        let full_key = format!("{}{}", self.prefix, key.to_uppercase());
        self.variables.get(&full_key)
    }

    /// Get environment variable value with fallback
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Set environment variable (for testing)
    #[cfg(test)]
    pub fn set(&mut self, key: &str, value: &str) {
        let full_key = format!("{}{}", self.prefix, key.to_uppercase());
        self.variables.insert(full_key, value.to_string());
    }

    /// Parse environment variables into configuration overrides
    pub fn parse_overrides(&self) -> Result<EnvConfigOverrides> {
        let mut overrides = EnvConfigOverrides {
            log_level: None,
            config_path: None,
            sources: None,
            packages: None,
            builtin_only: None,
            cache_ttl_seconds: None,
            cache_size: None,
            verbose: None,
            data_dir: None,
            hot_reload: None,
        };

        // Parse log level
        if let Some(level) = self.get("LOG_LEVEL") {
            overrides.log_level = Some(level.clone());
        }

        // Parse config path
        if let Some(path) = self.get("CONFIG_PATH") {
            overrides.config_path = Some(path.clone());
        }

        // Parse sources (comma-separated)
        if let Some(sources) = self.get("SOURCES") {
            overrides.sources = Some(sources.clone());
        }

        // Parse packages (comma-separated)
        if let Some(packages) = self.get("PACKAGES") {
            overrides.packages = Some(packages.clone());
        }

        // Parse builtin-only flag
        if let Some(builtin) = self.get("BUILTIN_ONLY") {
            overrides.builtin_only = Some(Self::parse_bool(builtin)?);
        }

        // Parse cache TTL
        if let Some(ttl) = self.get("CACHE_TTL_SECONDS") {
            overrides.cache_ttl_seconds = Some(
                ttl.parse()
                    .with_context(|| format!("Invalid cache TTL: {ttl}"))?,
            );
        }

        // Parse cache size
        if let Some(size) = self.get("CACHE_SIZE") {
            overrides.cache_size = Some(
                size.parse()
                    .with_context(|| format!("Invalid cache size: {size}"))?,
            );
        }

        // Parse verbose level
        if let Some(verbose) = self.get("VERBOSE") {
            overrides.verbose = Some(
                verbose
                    .parse()
                    .with_context(|| format!("Invalid verbose level: {verbose}"))?,
            );
        }

        // Parse data directory
        if let Some(data_dir) = self.get("DATA_DIR") {
            overrides.data_dir = Some(data_dir.clone());
        }

        // Parse hot-reload flag
        if let Some(hot_reload) = self.get("HOT_RELOAD") {
            overrides.hot_reload = Some(Self::parse_bool(hot_reload)?);
        }

        Ok(overrides)
    }

    /// Apply environment overrides to configuration
    pub fn apply_overrides_to_config(
        &self,
        mut config: SantaConfig,
        overrides: &EnvConfigOverrides,
    ) -> Result<SantaConfig> {
        info!("Applying environment variable overrides to configuration");

        // Override sources if provided
        if let Some(ref sources_str) = overrides.sources {
            let sources = Self::parse_sources(sources_str)?;
            if !sources.is_empty() {
                config.sources = sources;
                info!("Overrode sources from environment: {:?}", config.sources);
            }
        }

        // Override packages if provided
        if let Some(ref packages_str) = overrides.packages {
            let packages = Self::parse_packages(packages_str);
            if !packages.is_empty() {
                config.packages = packages;
                info!("Overrode packages from environment: {:?}", config.packages);
            }
        }

        // Override log level if provided
        if let Some(ref verbose_str) = overrides.verbose {
            config.log_level = *verbose_str;
            info!("Overrode log level from environment: {}", config.log_level);
        }

        Ok(config)
    }

    /// Parse boolean value from string
    fn parse_bool(value: &str) -> Result<bool> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
            "false" | "0" | "no" | "off" | "disabled" => Ok(false),
            _ => Err(anyhow::anyhow!("Invalid boolean value: '{}'. Use true/false, 1/0, yes/no, on/off, or enabled/disabled", value))
        }
    }

    /// Parse comma-separated sources string
    fn parse_sources(sources_str: &str) -> Result<Vec<KnownSources>> {
        let mut sources = Vec::new();

        for source_name in sources_str.split(',').map(|s| s.trim()) {
            if source_name.is_empty() {
                continue;
            }

            let source = match source_name.to_lowercase().as_str() {
                "apt" => KnownSources::Apt,
                "aur" => KnownSources::Aur,
                "brew" => KnownSources::Brew,
                "cargo" => KnownSources::Cargo,
                "pacman" => KnownSources::Pacman,
                "scoop" => KnownSources::Scoop,
                "nix" => KnownSources::Nix,
                _ => {
                    warn!("Unknown source '{}', treating as custom", source_name);
                    KnownSources::Unknown(source_name.to_string())
                }
            };

            sources.push(source);
        }

        Ok(sources)
    }

    /// Parse comma-separated packages string
    fn parse_packages(packages_str: &str) -> Vec<String> {
        packages_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Get all supported environment variables with descriptions
    pub fn get_supported_variables(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        let prefix = &self.prefix;

        vars.insert(
            format!("{prefix}LOG_LEVEL"),
            "Set log level (trace, debug, info, warn, error)".to_string(),
        );
        vars.insert(
            format!("{prefix}CONFIG_PATH"),
            "Override path to configuration file".to_string(),
        );
        vars.insert(
            format!("{prefix}SOURCES"),
            "Override package sources (comma-separated: brew,cargo,apt)".to_string(),
        );
        vars.insert(
            format!("{prefix}PACKAGES"),
            "Override package list (comma-separated)".to_string(),
        );
        vars.insert(
            format!("{prefix}BUILTIN_ONLY"),
            "Use builtin configuration only (true/false)".to_string(),
        );
        vars.insert(
            format!("{prefix}CACHE_TTL_SECONDS"),
            "Set package cache TTL in seconds".to_string(),
        );
        vars.insert(
            format!("{prefix}CACHE_SIZE"),
            "Set maximum cache size (number of entries)".to_string(),
        );
        vars.insert(
            format!("{prefix}VERBOSE"),
            "Set verbose logging level (0-3)".to_string(),
        );
        vars.insert(
            format!("{prefix}DATA_DIR"),
            "Override data directory path".to_string(),
        );
        vars.insert(
            format!("{prefix}HOT_RELOAD"),
            "Enable configuration hot-reloading (true/false)".to_string(),
        );

        vars
    }

    /// Print environment configuration help
    pub fn print_env_help(&self) {
        println!("Santa Environment Variables:");
        println!("============================");

        for (var_name, description) in self.get_supported_variables() {
            println!("  {var_name:<25} {description}");
        }

        println!("\nExamples:");
        println!("  export {}SOURCES=brew,cargo", self.prefix);
        println!("  export {}PACKAGES=git,rust,ripgrep", self.prefix);
        println!("  export {}LOG_LEVEL=debug", self.prefix);
        println!("  export {}BUILTIN_ONLY=true", self.prefix);
    }
}

/// Load configuration with environment variable support
pub fn load_config_with_env(config_path: Option<&str>, builtin_only: bool) -> Result<SantaConfig> {
    let mut env_config = EnvironmentConfig::default();
    env_config.load_from_env()?;

    let overrides = env_config.parse_overrides()?;

    // Determine config path (env var takes precedence)
    let actual_config_path = overrides
        .config_path
        .as_deref()
        .or(config_path)
        .unwrap_or("~/.config/santa/config.yaml");

    // Determine if builtin-only (env var takes precedence)
    let actual_builtin_only = overrides.builtin_only.unwrap_or(builtin_only);

    // Load base configuration
    let base_config = if actual_builtin_only {
        info!("Using builtin configuration (overridden by environment)");
        SantaConfig::default_for_platform()
    } else {
        info!("Loading configuration from: {}", actual_config_path);
        SantaConfig::load_from(std::path::Path::new(actual_config_path)).unwrap_or_else(|e| {
            warn!("Failed to load config file: {}. Using defaults.", e);
            SantaConfig::default_for_platform()
        })
    };

    // Apply environment overrides
    let final_config = env_config.apply_overrides_to_config(base_config, &overrides)?;

    info!("Configuration loaded with environment overrides applied");
    Ok(final_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_config_creation() {
        let env_config = EnvironmentConfig::default();
        assert_eq!(env_config.prefix, "SANTA_");
        assert!(env_config.variables.is_empty());

        let custom_env = EnvironmentConfig::with_prefix("TEST_");
        assert_eq!(custom_env.prefix, "TEST_");
    }

    #[test]
    fn test_boolean_parsing() {
        assert!(EnvironmentConfig::parse_bool("true").unwrap());
        assert!(EnvironmentConfig::parse_bool("1").unwrap());
        assert!(EnvironmentConfig::parse_bool("yes").unwrap());
        assert!(EnvironmentConfig::parse_bool("on").unwrap());
        assert!(EnvironmentConfig::parse_bool("enabled").unwrap());

        assert!(!EnvironmentConfig::parse_bool("false").unwrap());
        assert!(!EnvironmentConfig::parse_bool("0").unwrap());
        assert!(!EnvironmentConfig::parse_bool("no").unwrap());
        assert!(!EnvironmentConfig::parse_bool("off").unwrap());
        assert!(!EnvironmentConfig::parse_bool("disabled").unwrap());

        assert!(EnvironmentConfig::parse_bool("maybe").is_err());
        assert!(EnvironmentConfig::parse_bool("").is_err());
    }

    #[test]
    fn test_sources_parsing() {
        let sources = EnvironmentConfig::parse_sources("brew,cargo,apt").unwrap();
        assert_eq!(sources.len(), 3);
        assert!(sources.contains(&KnownSources::Brew));
        assert!(sources.contains(&KnownSources::Cargo));
        assert!(sources.contains(&KnownSources::Apt));

        let sources_with_spaces = EnvironmentConfig::parse_sources("brew, cargo , apt ").unwrap();
        assert_eq!(sources_with_spaces.len(), 3);

        let empty_sources = EnvironmentConfig::parse_sources("").unwrap();
        assert!(empty_sources.is_empty());

        let unknown_source = EnvironmentConfig::parse_sources("brew,unknown,cargo").unwrap();
        assert_eq!(unknown_source.len(), 3);
        assert!(matches!(unknown_source[1], KnownSources::Unknown(_)));
    }

    #[test]
    fn test_packages_parsing() {
        let packages = EnvironmentConfig::parse_packages("git,rust,ripgrep");
        assert_eq!(packages.len(), 3);
        assert!(packages.contains(&"git".to_string()));
        assert!(packages.contains(&"rust".to_string()));
        assert!(packages.contains(&"ripgrep".to_string()));

        let empty_packages = EnvironmentConfig::parse_packages("");
        assert!(empty_packages.is_empty());

        let packages_with_spaces = EnvironmentConfig::parse_packages("git, rust , ripgrep ");
        assert_eq!(packages_with_spaces.len(), 3);
        assert!(!packages_with_spaces[0].contains(' '));
    }

    #[test]
    fn test_environment_variable_access() {
        let mut env_config = EnvironmentConfig::default();
        env_config.set("LOG_LEVEL", "debug");
        env_config.set("SOURCES", "brew,cargo");

        assert_eq!(env_config.get("LOG_LEVEL"), Some(&"debug".to_string()));
        assert_eq!(env_config.get("SOURCES"), Some(&"brew,cargo".to_string()));
        assert_eq!(env_config.get("NONEXISTENT"), None);

        assert_eq!(env_config.get_or("NONEXISTENT", "default"), "default");
        assert_eq!(env_config.get_or("LOG_LEVEL", "default"), "debug");
    }

    #[test]
    fn test_override_parsing() {
        let mut env_config = EnvironmentConfig::default();
        env_config.set("LOG_LEVEL", "debug");
        env_config.set("SOURCES", "brew,cargo");
        env_config.set("PACKAGES", "git,rust");
        env_config.set("BUILTIN_ONLY", "true");
        env_config.set("VERBOSE", "2");
        env_config.set("CACHE_TTL_SECONDS", "300");

        let overrides = env_config.parse_overrides().unwrap();

        assert_eq!(overrides.log_level, Some("debug".to_string()));
        assert_eq!(overrides.sources, Some("brew,cargo".to_string()));
        assert_eq!(overrides.packages, Some("git,rust".to_string()));
        assert_eq!(overrides.builtin_only, Some(true));
        assert_eq!(overrides.verbose, Some(2));
        assert_eq!(overrides.cache_ttl_seconds, Some(300));
    }

    #[test]
    fn test_supported_variables_list() {
        let env_config = EnvironmentConfig::default();
        let vars = env_config.get_supported_variables();

        assert!(vars.contains_key("SANTA_LOG_LEVEL"));
        assert!(vars.contains_key("SANTA_CONFIG_PATH"));
        assert!(vars.contains_key("SANTA_SOURCES"));
        assert!(vars.contains_key("SANTA_PACKAGES"));
        assert!(vars.contains_key("SANTA_BUILTIN_ONLY"));

        // Check that descriptions are provided
        for (_, description) in vars {
            assert!(!description.is_empty());
        }
    }

    #[test]
    fn test_apply_overrides_to_config() {
        let env_config = EnvironmentConfig::default();
        let base_config = SantaConfig {
            sources: vec![KnownSources::Apt],
            packages: vec!["old-package".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let overrides = EnvConfigOverrides {
            sources: Some("brew,cargo".to_string()),
            packages: Some("git,rust".to_string()),
            verbose: Some(2),
            log_level: None,
            config_path: None,
            builtin_only: None,
            cache_ttl_seconds: None,
            cache_size: None,
            data_dir: None,
            hot_reload: None,
        };

        let result_config = env_config
            .apply_overrides_to_config(base_config, &overrides)
            .unwrap();

        assert_eq!(result_config.sources.len(), 2);
        assert!(result_config.sources.contains(&KnownSources::Brew));
        assert!(result_config.sources.contains(&KnownSources::Cargo));

        assert_eq!(result_config.packages.len(), 2);
        assert!(result_config.packages.contains(&"git".to_string()));
        assert!(result_config.packages.contains(&"rust".to_string()));

        assert_eq!(result_config.log_level, 2);
    }
}

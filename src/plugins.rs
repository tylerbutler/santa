use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::configuration::SantaConfig;
use crate::data::SantaData;

/// Plugin system foundation for Santa
/// 
/// This provides a basic plugin architecture that can be extended in the future
/// to support custom package sources, transformations, and behaviors.

/// Plugin metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Plugin entry point (for future dynamic loading)
    pub entry_point: Option<String>,
    /// Plugin dependencies
    pub dependencies: Vec<String>,
    /// Minimum Santa version required
    pub min_santa_version: Option<String>,
    /// Plugin tags/categories
    pub tags: Vec<String>,
}

/// Plugin lifecycle hooks
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Initialize the plugin
    fn initialize(&mut self, config: &SantaConfig, data: &SantaData) -> Result<()>;
    
    /// Called before command execution
    fn before_command(&self, command: &str, args: &[String]) -> Result<()>;
    
    /// Called after command execution
    fn after_command(&self, command: &str, args: &[String], result: &Result<()>) -> Result<()>;
    
    /// Called when configuration changes (if hot-reloading is enabled)
    fn on_config_change(&self, new_config: &SantaConfig) -> Result<()>;
    
    /// Shutdown the plugin
    fn shutdown(&mut self) -> Result<()>;
}

/// Plugin manager for handling plugin lifecycle
pub struct PluginManager {
    /// Registered plugins
    plugins: HashMap<String, Box<dyn Plugin>>,
    /// Plugin directories to search
    plugin_dirs: Vec<PathBuf>,
    /// Whether plugins are enabled
    enabled: bool,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dirs: vec![
                // Standard plugin directories
                PathBuf::from("~/.config/santa/plugins"),
                PathBuf::from("/usr/local/share/santa/plugins"),
                PathBuf::from("./plugins"), // Development directory
            ],
            enabled: true,
        }
    }

    /// Enable or disable the plugin system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            info!("Plugin system disabled");
        }
    }

    /// Check if plugins are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Add a plugin directory to search
    pub fn add_plugin_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.plugin_dirs.push(dir.as_ref().to_path_buf());
    }

    /// Register a plugin manually (useful for built-in plugins)
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let name = plugin.metadata().name.clone();
        info!("Registering plugin: {}", name);
        
        self.plugins.insert(name, plugin);
        Ok(())
    }

    /// Initialize all registered plugins
    pub fn initialize_plugins(&mut self, config: &SantaConfig, data: &SantaData) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        info!("Initializing {} plugins", self.plugins.len());
        
        for (name, plugin) in &mut self.plugins {
            debug!("Initializing plugin: {}", name);
            plugin.initialize(config, data)
                .with_context(|| format!("Failed to initialize plugin: {}", name))?;
        }
        
        Ok(())
    }

    /// Call before_command hook on all plugins
    pub fn before_command(&self, command: &str, args: &[String]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Calling before_command hooks for: {} {:?}", command, args);
        
        for (name, plugin) in &self.plugins {
            plugin.before_command(command, args)
                .with_context(|| format!("Plugin '{}' before_command hook failed", name))?;
        }
        
        Ok(())
    }

    /// Call after_command hook on all plugins
    pub fn after_command(&self, command: &str, args: &[String], result: &Result<()>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Calling after_command hooks for: {} {:?}", command, args);
        
        for (name, plugin) in &self.plugins {
            plugin.after_command(command, args, result)
                .with_context(|| format!("Plugin '{}' after_command hook failed", name))?;
        }
        
        Ok(())
    }

    /// Call on_config_change hook on all plugins
    pub fn on_config_change(&self, new_config: &SantaConfig) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Calling on_config_change hooks");
        
        for (name, plugin) in &self.plugins {
            plugin.on_config_change(new_config)
                .with_context(|| format!("Plugin '{}' on_config_change hook failed", name))?;
        }
        
        Ok(())
    }

    /// Shutdown all plugins
    pub fn shutdown_plugins(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        info!("Shutting down {} plugins", self.plugins.len());
        
        for (name, plugin) in &mut self.plugins {
            debug!("Shutting down plugin: {}", name);
            plugin.shutdown()
                .with_context(|| format!("Failed to shutdown plugin: {}", name))?;
        }
        
        Ok(())
    }

    /// Get list of registered plugins
    pub fn list_plugins(&self) -> Vec<&PluginMetadata> {
        self.plugins.values().map(|p| p.metadata()).collect()
    }

    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }

    /// Discover plugins in plugin directories (placeholder for future implementation)
    pub fn discover_plugins(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        info!("Plugin discovery is not yet implemented");
        info!("Plugin directories: {:?}", self.plugin_dirs);
        
        // In the future, this would:
        // 1. Scan plugin directories for plugin manifests
        // 2. Load plugin metadata
        // 3. Dynamically load plugin libraries (if supported)
        // 4. Register discovered plugins
        
        Ok(())
    }

    /// Create plugin manager with configuration
    pub fn with_config(_config: &SantaConfig) -> Self {
        let manager = Self::new();
        
        // In the future, this could read plugin settings from config:
        // - Disabled/enabled plugins
        // - Plugin directories
        // - Plugin-specific configuration
        
        manager
    }
}

/// Built-in plugin for logging command execution
#[derive(Debug)]
pub struct LoggingPlugin {
    metadata: PluginMetadata,
    enabled: bool,
}

impl LoggingPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "logging".to_string(),
                version: "1.0.0".to_string(),
                description: "Logs command execution for debugging and auditing".to_string(),
                author: "Santa Team".to_string(),
                entry_point: None,
                dependencies: vec![],
                min_santa_version: Some("0.1.0".to_string()),
                tags: vec!["logging".to_string(), "audit".to_string()],
            },
            enabled: true,
        }
    }
}

impl Plugin for LoggingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, _config: &SantaConfig, _data: &SantaData) -> Result<()> {
        info!("Logging plugin initialized");
        Ok(())
    }

    fn before_command(&self, command: &str, args: &[String]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        info!("Executing command: {} {:?}", command, args);
        Ok(())
    }

    fn after_command(&self, command: &str, args: &[String], result: &Result<()>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        match result {
            Ok(_) => info!("Command completed successfully: {} {:?}", command, args),
            Err(e) => warn!("Command failed: {} {:?} - Error: {}", command, args, e),
        }
        Ok(())
    }

    fn on_config_change(&self, _new_config: &SantaConfig) -> Result<()> {
        debug!("Logging plugin notified of configuration change");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        info!("Logging plugin shutdown");
        Ok(())
    }
}

/// Built-in plugin for performance monitoring
#[derive(Debug)]
pub struct PerformancePlugin {
    metadata: PluginMetadata,
    start_time: Option<std::time::Instant>,
}

impl PerformancePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "performance".to_string(),
                version: "1.0.0".to_string(),
                description: "Monitors command execution performance".to_string(),
                author: "Santa Team".to_string(),
                entry_point: None,
                dependencies: vec![],
                min_santa_version: Some("0.1.0".to_string()),
                tags: vec!["performance".to_string(), "monitoring".to_string()],
            },
            start_time: None,
        }
    }
}

impl Plugin for PerformancePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, _config: &SantaConfig, _data: &SantaData) -> Result<()> {
        info!("Performance monitoring plugin initialized");
        Ok(())
    }

    fn before_command(&self, _command: &str, _args: &[String]) -> Result<()> {
        // Note: We can't modify self here due to &self, so timing would need
        // to be handled differently in a real implementation (e.g., thread-local storage)
        debug!("Performance monitoring: Command started");
        Ok(())
    }

    fn after_command(&self, command: &str, _args: &[String], result: &Result<()>) -> Result<()> {
        if result.is_ok() {
            debug!("Performance monitoring: Command '{}' completed", command);
        } else {
            debug!("Performance monitoring: Command '{}' failed", command);
        }
        Ok(())
    }

    fn on_config_change(&self, _new_config: &SantaConfig) -> Result<()> {
        debug!("Performance plugin notified of configuration change");
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        info!("Performance monitoring plugin shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KnownSources;

    fn create_test_config() -> SantaConfig {
        SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        }
    }

    fn create_test_data() -> SantaData {
        SantaData::default()
    }

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(manager.is_enabled());
        assert!(manager.plugins.is_empty());
        assert!(!manager.plugin_dirs.is_empty());
    }

    #[test]
    fn test_plugin_manager_enable_disable() {
        let mut manager = PluginManager::new();
        assert!(manager.is_enabled());
        
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        
        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_plugin_registration() {
        let mut manager = PluginManager::new();
        let plugin = Box::new(LoggingPlugin::new());
        
        assert!(manager.register_plugin(plugin).is_ok());
        assert_eq!(manager.plugins.len(), 1);
        assert!(manager.get_plugin("logging").is_some());
    }

    #[test]
    fn test_plugin_registration_when_disabled() {
        let mut manager = PluginManager::new();
        manager.set_enabled(false);
        
        let plugin = Box::new(LoggingPlugin::new());
        assert!(manager.register_plugin(plugin).is_ok());
        // Plugin should not be added when system is disabled
        assert!(manager.plugins.is_empty());
    }

    #[test]
    fn test_plugin_initialization() {
        let mut manager = PluginManager::new();
        let plugin = Box::new(LoggingPlugin::new());
        
        manager.register_plugin(plugin).unwrap();
        
        let config = create_test_config();
        let data = create_test_data();
        
        assert!(manager.initialize_plugins(&config, &data).is_ok());
    }

    #[test]
    fn test_plugin_hooks() {
        let mut manager = PluginManager::new();
        let plugin = Box::new(LoggingPlugin::new());
        
        manager.register_plugin(plugin).unwrap();
        
        let config = create_test_config();
        let data = create_test_data();
        manager.initialize_plugins(&config, &data).unwrap();
        
        // Test command hooks
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        assert!(manager.before_command("test", &args).is_ok());
        
        let result: Result<()> = Ok(());
        assert!(manager.after_command("test", &args, &result).is_ok());
        
        // Test config change hook
        assert!(manager.on_config_change(&config).is_ok());
        
        // Test shutdown
        assert!(manager.shutdown_plugins().is_ok());
    }

    #[test]
    fn test_plugin_list() {
        let mut manager = PluginManager::new();
        let logging_plugin = Box::new(LoggingPlugin::new());
        let perf_plugin = Box::new(PerformancePlugin::new());
        
        manager.register_plugin(logging_plugin).unwrap();
        manager.register_plugin(perf_plugin).unwrap();
        
        let plugin_list = manager.list_plugins();
        assert_eq!(plugin_list.len(), 2);
        
        let plugin_names: Vec<&str> = plugin_list.iter().map(|p| p.name.as_str()).collect();
        assert!(plugin_names.contains(&"logging"));
        assert!(plugin_names.contains(&"performance"));
    }

    #[test]
    fn test_built_in_plugins_metadata() {
        let logging_plugin = LoggingPlugin::new();
        let metadata = logging_plugin.metadata();
        
        assert_eq!(metadata.name, "logging");
        assert_eq!(metadata.version, "1.0.0");
        assert!(!metadata.description.is_empty());
        assert!(!metadata.author.is_empty());
        assert!(metadata.tags.contains(&"logging".to_string()));
        
        let perf_plugin = PerformancePlugin::new();
        let perf_metadata = perf_plugin.metadata();
        
        assert_eq!(perf_metadata.name, "performance");
        assert_eq!(perf_metadata.version, "1.0.0");
        assert!(perf_metadata.tags.contains(&"performance".to_string()));
    }

    #[test]
    fn test_plugin_manager_with_config() {
        let config = create_test_config();
        let manager = PluginManager::with_config(&config);
        
        assert!(manager.is_enabled());
        assert!(manager.plugins.is_empty()); // No plugins auto-registered yet
    }

    #[test]
    fn test_plugin_discovery_placeholder() {
        let mut manager = PluginManager::new();
        
        // This should not fail even though discovery is not implemented
        assert!(manager.discover_plugins().is_ok());
    }

    #[test]
    fn test_plugin_add_directory() {
        let mut manager = PluginManager::new();
        let initial_count = manager.plugin_dirs.len();
        
        manager.add_plugin_dir("/custom/plugin/path");
        assert_eq!(manager.plugin_dirs.len(), initial_count + 1);
        assert!(manager.plugin_dirs.contains(&PathBuf::from("/custom/plugin/path")));
    }
}
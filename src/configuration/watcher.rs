use crate::configuration::SantaConfig;
use crate::data::SantaData;
use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Configuration change notification
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    pub path: PathBuf,
    pub config: SantaConfig,
}

/// Configuration watcher service for hot-reloading
pub struct ConfigWatcher {
    config_path: PathBuf,
    current_config: Arc<RwLock<SantaConfig>>,
    change_sender: broadcast::Sender<ConfigChangeEvent>,
    _watcher: Option<RecommendedWatcher>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new(config_path: PathBuf, initial_config: SantaConfig) -> Result<Self> {
        let (change_sender, _) = broadcast::channel(100);

        Ok(ConfigWatcher {
            config_path,
            current_config: Arc::new(RwLock::new(initial_config)),
            change_sender,
            _watcher: None,
        })
    }

    /// Get the current configuration
    pub async fn current_config(&self) -> SantaConfig {
        self.current_config.read().await.clone()
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.change_sender.subscribe()
    }

    /// Start watching for configuration file changes
    pub async fn start_watching(&mut self, data: Arc<SantaData>) -> Result<()> {
        if !self.config_path.exists() {
            info!(
                "Config file does not exist, creating parent directory if needed: {}",
                self.config_path.display()
            );
            if let Some(parent) = self.config_path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create config directory: {}", parent.display())
                })?;
            }
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let config_path = self.config_path.clone();

        // Set up file watcher
        let mut watcher = notify::recommended_watcher(
            move |result: Result<Event, notify::Error>| match result {
                Ok(event) => {
                    if let Err(e) = tx.blocking_send(event) {
                        error!("Failed to send file watch event: {}", e);
                    }
                }
                Err(e) => error!("File watch error: {}", e),
            },
        )?;

        // Watch the config file and its parent directory
        if let Some(parent_dir) = config_path.parent() {
            watcher
                .watch(parent_dir, RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch directory: {}", parent_dir.display()))?;
        } else {
            watcher
                .watch(&config_path, RecursiveMode::NonRecursive)
                .with_context(|| format!("Failed to watch file: {}", config_path.display()))?;
        }

        self._watcher = Some(watcher);

        // Spawn background task to handle file change events
        let current_config = Arc::clone(&self.current_config);
        let change_sender = self.change_sender.clone();
        let watch_path = self.config_path.clone();

        tokio::spawn(async move {
            info!(
                "Started configuration file watcher for: {}",
                watch_path.display()
            );

            while let Some(event) = rx.recv().await {
                if Self::is_config_change_event(&event, &watch_path) {
                    debug!("Config file change detected: {:?}", event);

                    // Add a small delay to allow file write to complete
                    sleep(Duration::from_millis(100)).await;

                    match Self::reload_config(&watch_path, &data).await {
                        Ok(new_config) => {
                            // Update current config
                            *current_config.write().await = new_config.clone();

                            // Notify subscribers
                            let change_event = ConfigChangeEvent {
                                path: watch_path.clone(),
                                config: new_config,
                            };

                            if let Err(e) = change_sender.send(change_event) {
                                warn!("No subscribers for config change event: {}", e);
                            }

                            info!(
                                "Configuration reloaded successfully from: {}",
                                watch_path.display()
                            );
                        }
                        Err(e) => {
                            error!("Failed to reload configuration: {}", e);
                            // Don't update the config if reload failed
                        }
                    }
                }
            }

            info!("Configuration watcher stopped");
        });

        Ok(())
    }

    /// Check if the event is a configuration file change we care about
    fn is_config_change_event(event: &Event, config_path: &Path) -> bool {
        match event.kind {
            notify::EventKind::Modify(_) | notify::EventKind::Create(_) => event
                .paths
                .iter()
                .any(|path| path == config_path || path.file_name() == config_path.file_name()),
            _ => false,
        }
    }

    /// Safely reload configuration with validation
    async fn reload_config(config_path: &Path, data: &SantaData) -> Result<SantaConfig> {
        // Load new configuration
        let new_config = SantaConfig::load_from(config_path)
            .with_context(|| format!("Failed to load config from: {}", config_path.display()))?;

        // Validate with current data
        new_config
            .validate_with_data(data)
            .with_context(|| "New configuration validation failed")?;

        debug!(
            "Configuration validation passed for: {}",
            config_path.display()
        );
        Ok(new_config)
    }

    /// Update the configuration manually (useful for testing)
    pub async fn update_config(&self, new_config: SantaConfig) -> Result<()> {
        *self.current_config.write().await = new_config.clone();

        let change_event = ConfigChangeEvent {
            path: self.config_path.clone(),
            config: new_config,
        };

        self.change_sender
            .send(change_event)
            .map_err(|e| anyhow::anyhow!("Failed to send config update event: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KnownSources;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_config_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let initial_config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let watcher = ConfigWatcher::new(config_path, initial_config.clone()).unwrap();
        let current = watcher.current_config().await;

        assert_eq!(current.sources, initial_config.sources);
        assert_eq!(current.packages, initial_config.packages);
    }

    #[tokio::test]
    async fn test_config_manual_update() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let initial_config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let watcher = ConfigWatcher::new(config_path, initial_config).unwrap();
        let mut receiver = watcher.subscribe();

        let new_config = SantaConfig {
            sources: vec![KnownSources::Cargo],
            packages: vec!["rust".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        // Update config manually
        watcher.update_config(new_config.clone()).await.unwrap();

        // Check that current config was updated
        let current = watcher.current_config().await;
        assert_eq!(current.sources, vec![KnownSources::Cargo]);
        assert_eq!(current.packages, vec!["rust".to_string()]);

        // Check that change event was sent
        let change_event = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(change_event.config.sources, vec![KnownSources::Cargo]);
    }

    #[tokio::test]
    async fn test_is_config_change_event() {
        let config_path = Path::new("/tmp/config.yaml");

        // Test modify event
        let modify_event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![config_path.to_path_buf()],
            attrs: Default::default(),
        };

        assert!(ConfigWatcher::is_config_change_event(
            &modify_event,
            config_path
        ));

        // Test create event
        let create_event = Event {
            kind: notify::EventKind::Create(notify::event::CreateKind::File),
            paths: vec![config_path.to_path_buf()],
            attrs: Default::default(),
        };

        assert!(ConfigWatcher::is_config_change_event(
            &create_event,
            config_path
        ));

        // Test irrelevant event
        let other_event = Event {
            kind: notify::EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![Path::new("/tmp/other.txt").to_path_buf()],
            attrs: Default::default(),
        };

        assert!(!ConfigWatcher::is_config_change_event(
            &other_event,
            config_path
        ));
    }

    #[tokio::test]
    async fn test_config_reload_validation() {
        let valid_config = r#"
sources: ["brew", "cargo"]
packages: ["git", "rust"]
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{valid_config}").unwrap();
        temp_file.flush().unwrap();

        let data = SantaData::default();
        let result = ConfigWatcher::reload_config(temp_file.path(), &data).await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.packages.len(), 2);
    }

    #[tokio::test]
    async fn test_config_reload_invalid_yaml() {
        let invalid_config = "invalid: yaml: content: [";

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{invalid_config}").unwrap();
        temp_file.flush().unwrap();

        let data = SantaData::default();
        let result = ConfigWatcher::reload_config(temp_file.path(), &data).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to load config"));
    }

    #[tokio::test]
    async fn test_config_reload_validation_failure() {
        let invalid_config = r#"
sources: []
packages: ["git"]
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{invalid_config}").unwrap();
        temp_file.flush().unwrap();

        let data = SantaData::default();
        let result = ConfigWatcher::reload_config(temp_file.path(), &data).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // The error should contain information about failure to load the config
        // (which includes validation failure in the chain)
        assert!(
            error_msg.contains("Failed to load config from")
                || error_msg.contains("At least one source must be configured")
        );
    }
}

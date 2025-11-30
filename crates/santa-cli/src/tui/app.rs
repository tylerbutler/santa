//! Application state for the TUI.
//!
//! This module manages the TUI state, including navigation,
//! package data, and UI focus.

use crate::configuration::{SantaConfig, SantaConfigExt};
use crate::data::{SantaData, SourceList};
use crate::errors::Result;
use crate::sources::{PackageCache, PackageSource};
use futures::future::try_join_all;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::debug;

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// Source list panel (left side)
    #[default]
    SourceList,
    /// Package list panel (right side)
    PackageList,
}

/// Information about a package for display.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Whether the package is installed
    pub installed: bool,
}

/// Group of packages for a source.
#[derive(Debug, Clone)]
pub struct SourceGroup {
    /// The source this group belongs to
    pub source: PackageSource,
    /// Packages in this group
    pub packages: Vec<PackageInfo>,
}

impl SourceGroup {
    /// Count of installed packages.
    pub fn installed_count(&self) -> usize {
        self.packages.iter().filter(|p| p.installed).count()
    }

    /// Count of missing packages.
    pub fn missing_count(&self) -> usize {
        self.packages.iter().filter(|p| !p.installed).count()
    }

    /// Total package count.
    pub fn total_count(&self) -> usize {
        self.packages.len()
    }
}

/// Application state for the TUI dashboard.
pub struct App {
    // Data layer
    config: SantaConfig,
    data: SantaData,
    cache: PackageCache,

    // Computed data (cached for display)
    /// List of enabled sources
    pub sources: SourceList,
    /// Package groups by source
    pub source_groups: Vec<SourceGroup>,

    // UI State
    /// Currently selected source index
    pub selected_source_index: usize,
    /// Currently selected package index within the selected source
    pub selected_package_index: usize,
    /// Set of expanded source indices
    pub expanded_sources: HashSet<usize>,
    /// Which panel has focus
    pub focus: Focus,
    /// Whether to show help overlay
    pub show_help: bool,

    // Status
    /// Whether data is currently loading
    pub is_loading: bool,
    /// Last refresh timestamp
    pub last_refresh: Option<Instant>,
    /// Status message to display
    pub status_message: Option<String>,

    // App lifecycle
    /// Whether the app should quit
    pub should_quit: bool,
}

impl App {
    /// Create a new App with the given configuration and data.
    pub fn new(config: SantaConfig, data: SantaData, cache: PackageCache) -> Self {
        Self {
            config,
            data,
            cache,
            sources: Vec::new(),
            source_groups: Vec::new(),
            selected_source_index: 0,
            selected_package_index: 0,
            expanded_sources: HashSet::new(),
            focus: Focus::default(),
            show_help: false,
            is_loading: false,
            last_refresh: None,
            status_message: None,
            should_quit: false,
        }
    }

    /// Refresh package data from all sources.
    ///
    /// This mirrors the logic in `status_command` but stores results in app state.
    pub async fn refresh_data(&mut self) -> Result<()> {
        self.is_loading = true;
        self.status_message = Some("Refreshing...".to_string());

        // Filter sources to those enabled in the config
        self.sources = self
            .data
            .sources
            .iter()
            .filter(|source| self.config.source_is_enabled(source))
            .cloned()
            .collect();

        // Use structured concurrency to cache data for all sources concurrently
        let cache = Arc::new(RwLock::new(std::mem::take(&mut self.cache)));
        let cache_tasks: Vec<_> = self
            .sources
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

        // Wait for all cache operations
        match try_join_all(cache_tasks).await {
            Ok(_) => debug!("Successfully cached data for all sources"),
            Err(e) => {
                self.status_message = Some(format!("Some cache operations failed: {}", e));
            }
        }

        // Extract cache from Arc<RwLock<>>
        self.cache = Arc::try_unwrap(cache)
            .map_err(|_| {
                crate::errors::SantaError::Concurrency(
                    "Failed to unwrap cache - still in use".to_string(),
                )
            })?
            .into_inner();

        // Build source groups
        self.source_groups.clear();
        for source in &self.sources {
            let groups = self.config.groups(&self.data);
            for (key, pkgs) in groups {
                if source.name() == &key {
                    let packages: Vec<PackageInfo> = pkgs
                        .iter()
                        .map(|pkg| PackageInfo {
                            name: pkg.clone(),
                            installed: self.cache.check(source, pkg),
                        })
                        .collect();

                    self.source_groups.push(SourceGroup {
                        source: source.clone(),
                        packages,
                    });
                    break;
                }
            }
        }

        self.is_loading = false;
        self.last_refresh = Some(Instant::now());
        self.status_message = None;

        // Ensure selection is valid
        if self.selected_source_index >= self.source_groups.len() {
            self.selected_source_index = 0;
        }

        Ok(())
    }

    /// Get the currently selected source group.
    pub fn selected_source_group(&self) -> Option<&SourceGroup> {
        self.source_groups.get(self.selected_source_index)
    }

    /// Check if the source at the given index is expanded.
    pub fn is_expanded(&self, index: usize) -> bool {
        self.expanded_sources.contains(&index)
    }

    /// Toggle expansion of the currently selected source.
    pub fn toggle_expand(&mut self) {
        if self.focus == Focus::SourceList {
            if self.expanded_sources.contains(&self.selected_source_index) {
                self.expanded_sources.remove(&self.selected_source_index);
            } else {
                self.expanded_sources.insert(self.selected_source_index);
            }
        }
    }

    /// Move selection to the next source.
    pub fn next_source(&mut self) {
        if !self.source_groups.is_empty() {
            self.selected_source_index =
                (self.selected_source_index + 1) % self.source_groups.len();
            self.selected_package_index = 0;
        }
    }

    /// Move selection to the previous source.
    pub fn prev_source(&mut self) {
        if !self.source_groups.is_empty() {
            self.selected_source_index = if self.selected_source_index == 0 {
                self.source_groups.len() - 1
            } else {
                self.selected_source_index - 1
            };
            self.selected_package_index = 0;
        }
    }

    /// Move selection to the next package.
    pub fn next_package(&mut self) {
        if let Some(group) = self.selected_source_group() {
            if !group.packages.is_empty() {
                self.selected_package_index =
                    (self.selected_package_index + 1) % group.packages.len();
            }
        }
    }

    /// Move selection to the previous package.
    pub fn prev_package(&mut self) {
        if let Some(group) = self.selected_source_group() {
            if !group.packages.is_empty() {
                self.selected_package_index = if self.selected_package_index == 0 {
                    group.packages.len() - 1
                } else {
                    self.selected_package_index - 1
                };
            }
        }
    }

    /// Handle up action based on current focus.
    pub fn handle_up(&mut self) {
        match self.focus {
            Focus::SourceList => self.prev_source(),
            Focus::PackageList => self.prev_package(),
        }
    }

    /// Handle down action based on current focus.
    pub fn handle_down(&mut self) {
        match self.focus {
            Focus::SourceList => self.next_source(),
            Focus::PackageList => self.next_package(),
        }
    }

    /// Handle left action - switch to source panel.
    pub fn handle_left(&mut self) {
        self.focus = Focus::SourceList;
    }

    /// Handle right action - switch to package panel.
    pub fn handle_right(&mut self) {
        self.focus = Focus::PackageList;
    }

    /// Switch focus between panels.
    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::SourceList => Focus::PackageList,
            Focus::PackageList => Focus::SourceList,
        };
    }

    /// Toggle help overlay.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Get human-readable time since last refresh.
    pub fn time_since_refresh(&self) -> String {
        match self.last_refresh {
            Some(instant) => {
                let secs = instant.elapsed().as_secs();
                if secs < 60 {
                    format!("{}s ago", secs)
                } else {
                    format!("{}m ago", secs / 60)
                }
            }
            None => "never".to_string(),
        }
    }

    /// Get summary statistics.
    pub fn summary(&self) -> (usize, usize, usize) {
        let total: usize = self.source_groups.iter().map(|g| g.total_count()).sum();
        let installed: usize = self.source_groups.iter().map(|g| g.installed_count()).sum();
        let missing = total - installed;
        (total, installed, missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let config = SantaConfig::default_for_platform();
        let data = SantaData::default();
        let cache = PackageCache::new();
        App::new(config, data, cache)
    }

    #[test]
    fn test_app_creation() {
        let app = create_test_app();
        assert!(!app.should_quit);
        assert!(!app.is_loading);
        assert_eq!(app.focus, Focus::SourceList);
    }

    #[test]
    fn test_focus_switching() {
        let mut app = create_test_app();
        assert_eq!(app.focus, Focus::SourceList);

        app.switch_focus();
        assert_eq!(app.focus, Focus::PackageList);

        app.switch_focus();
        assert_eq!(app.focus, Focus::SourceList);
    }

    #[test]
    fn test_toggle_help() {
        let mut app = create_test_app();
        assert!(!app.show_help);

        app.toggle_help();
        assert!(app.show_help);

        app.toggle_help();
        assert!(!app.show_help);
    }

    #[test]
    fn test_expand_toggle() {
        let mut app = create_test_app();
        assert!(!app.is_expanded(0));

        app.toggle_expand();
        assert!(app.is_expanded(0));

        app.toggle_expand();
        assert!(!app.is_expanded(0));
    }
}

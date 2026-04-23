//! Application state for the Santa TUI.
//!
//! Manages package/source data, selection state, and user interactions.

use crate::configuration::{SantaConfig, SantaConfigExt};
use crate::data::{KnownSources, SantaData};
use crate::script_generator::{ExecutionMode, ScriptFormat};
use crate::sources::{PackageCache, PackageSource};
use futures::future::try_join_all;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Current phase of the TUI lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppPhase {
    /// Loading package data from sources.
    Loading,
    /// Main interactive view.
    Ready,
    /// Installing selected packages.
    Installing,
    /// Displaying a result message after an action.
    Message(String),
}

/// A row in the packages table, pre-computed for display.
#[derive(Debug, Clone)]
pub struct PackageRow {
    /// The canonical package name from config.
    pub name: String,
    /// Which source this package is assigned to.
    pub source: KnownSources,
    /// The emoji for the source.
    pub source_emoji: String,
    /// Whether the package is currently installed.
    pub installed: bool,
    /// The resolved name used by the source (may differ from canonical name).
    pub resolved_name: String,
}

/// Holds the full application state for the TUI.
pub struct App {
    /// Current lifecycle phase.
    pub phase: AppPhase,
    /// All package rows (computed after cache load).
    pub packages: Vec<PackageRow>,
    /// Indices of packages the user has selected for installation.
    pub selected: Vec<bool>,
    /// Currently highlighted row index.
    pub cursor: usize,
    /// Configured and enabled sources with availability info.
    pub sources: Vec<SourceInfo>,
    /// Whether the application should quit.
    pub should_quit: bool,

    // Internal data needed for install operations.
    config: SantaConfig,
    data: SantaData,
    cache: Option<PackageCache>,
    /// The execution mode (Safe or Execute) — exposed for UI hints.
    pub execution_mode: ExecutionMode,
    script_format: ScriptFormat,
    output_dir: PathBuf,
}

/// Info about a configured source for display.
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub name: KnownSources,
    pub emoji: String,
    /// Whether the underlying command (e.g. `brew`) is found on PATH.
    pub available: bool,
    /// Number of packages assigned to this source.
    pub package_count: usize,
    /// Number of installed packages for this source.
    pub installed_count: usize,
}

impl App {
    /// Create a new App with the given config/data/settings.
    pub fn new(
        config: SantaConfig,
        data: SantaData,
        execution_mode: ExecutionMode,
        script_format: ScriptFormat,
        output_dir: PathBuf,
    ) -> Self {
        Self {
            phase: AppPhase::Loading,
            packages: Vec::new(),
            selected: Vec::new(),
            cursor: 0,
            sources: Vec::new(),
            should_quit: false,
            config,
            data,
            cache: None,
            execution_mode,
            script_format,
            output_dir,
        }
    }

    /// Load package data: populate the cache, compute package rows and source info.
    pub async fn load_data(&mut self) -> anyhow::Result<()> {
        let enabled_sources: Vec<PackageSource> = self
            .data
            .sources
            .iter()
            .filter(|s| self.config.source_is_enabled(s))
            .cloned()
            .collect();

        // Populate cache concurrently
        let cache = PackageCache::new();
        let cache = Arc::new(RwLock::new(cache));
        let tasks: Vec<_> = enabled_sources
            .iter()
            .map(|source| {
                let cache_clone = Arc::clone(&cache);
                let source = source.clone();
                async move {
                    let c = cache_clone.write().await;
                    c.cache_for_async(&source).await
                }
            })
            .collect();

        match try_join_all(tasks).await {
            Ok(_) => debug!("TUI: cached data for all sources"),
            Err(e) => debug!("TUI: some cache operations failed: {e}"),
        }

        let cache = Arc::try_unwrap(cache)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap cache"))?
            .into_inner();

        // Build package rows from config groups
        let groups = self.config.groups(&self.data);
        let mut packages = Vec::new();

        for source in &enabled_sources {
            if let Some(pkgs) = groups.get(source.name()) {
                for pkg in pkgs {
                    let installed = cache.check(source, pkg, &self.data);
                    let resolved_name = self.data.name_for(pkg, source);
                    packages.push(PackageRow {
                        name: pkg.clone(),
                        source: source.name().clone(),
                        source_emoji: source.emoji().to_string(),
                        installed,
                        resolved_name,
                    });
                }
            }
        }

        // Sort: missing packages first, then alphabetical
        packages.sort_by(|a, b| {
            a.installed
                .cmp(&b.installed)
                .then_with(|| a.name.cmp(&b.name))
        });

        // Build source info
        let mut source_infos = Vec::new();
        for source in &enabled_sources {
            let available = which::which(source.shell_command().split_whitespace().next().unwrap_or(""))
                .is_ok();
            let assigned: Vec<&PackageRow> = packages
                .iter()
                .filter(|p| &p.source == source.name())
                .collect();
            let installed_count = assigned.iter().filter(|p| p.installed).count();
            source_infos.push(SourceInfo {
                name: source.name().clone(),
                emoji: source.emoji().to_string(),
                available,
                package_count: assigned.len(),
                installed_count,
            });
        }

        self.selected = vec![false; packages.len()];
        self.packages = packages;
        self.sources = source_infos;
        self.cache = Some(cache);
        self.phase = AppPhase::Ready;
        Ok(())
    }

    /// Toggle selection on the current row.
    pub fn toggle_selection(&mut self) {
        if let Some(sel) = self.selected.get_mut(self.cursor) {
            // Only allow selecting missing packages
            if !self.packages[self.cursor].installed {
                *sel = !*sel;
            }
        }
    }

    /// Select all missing packages.
    pub fn select_all_missing(&mut self) {
        for (i, pkg) in self.packages.iter().enumerate() {
            if !pkg.installed {
                self.selected[i] = true;
            }
        }
    }

    /// Deselect all packages.
    pub fn deselect_all(&mut self) {
        self.selected.fill(false);
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        if self.cursor + 1 < self.packages.len() {
            self.cursor += 1;
        }
    }

    /// Count how many packages are selected.
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|&&s| s).count()
    }

    /// Count how many packages are missing.
    pub fn missing_count(&self) -> usize {
        self.packages.iter().filter(|p| !p.installed).count()
    }

    /// Install selected packages. Returns a status message.
    pub fn install_selected(&mut self) -> anyhow::Result<String> {
        // Collect selected package info as owned data to avoid borrow conflicts
        let selected_info: Vec<(String, KnownSources)> = self
            .packages
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected[*i])
            .map(|(_, p)| (p.name.clone(), p.source.clone()))
            .collect();

        if selected_info.is_empty() {
            return Ok("No packages selected.".to_string());
        }

        let total = selected_info.len();

        // Group selected by source
        let mut by_source: HashMap<KnownSources, Vec<String>> = HashMap::new();
        for (name, source) in &selected_info {
            by_source
                .entry(source.clone())
                .or_default()
                .push(name.clone());
        }

        let enabled_sources: Vec<PackageSource> = self
            .data
            .sources
            .iter()
            .filter(|s| self.config.source_is_enabled(s))
            .cloned()
            .collect();

        let mut messages = Vec::new();

        for source in &enabled_sources {
            if let Some(pkgs) = by_source.get(source.name()) {
                source.exec_install(
                    &mut self.config,
                    &self.data,
                    pkgs.clone(),
                    self.execution_mode.clone(),
                    self.script_format.clone(),
                    &self.output_dir,
                )?;
                messages.push(format!(
                    "{} {}: {} package(s)",
                    source.emoji(),
                    source.name(),
                    pkgs.len()
                ));
            }
        }

        // Clear selection after install
        self.deselect_all();

        let mode_label = match self.execution_mode {
            ExecutionMode::Safe => "Script(s) generated",
            ExecutionMode::Execute => "Installed",
        };
        Ok(format!(
            "{mode_label} for {total} package(s): {}",
            messages.join(", ")
        ))
    }
}

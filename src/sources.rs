use crate::SantaConfig;
use std::collections::HashMap;

use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use subprocess::Exec;
use tabular::{Row, Table};

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
        let map: HashMap<String, Vec<String>> = HashMap::new();
        PackageCache { cache: map }
    }

    /// Checks for a package in the cache. This accesses the cache only, and will not modify it.
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

    /// Returns all packages for a PackageSource. This will call the PackageSource's check_command and populate the cache if needed.
    /// If the PackageSource can't be found, or the cache population fails, then None will be returned.
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SourceOverride {
    platform: Platform,
    pub shell_command: Option<String>,
    pub install_command: Option<String>,
    pub check_command: Option<String>,
}

impl SourceOverride {
    pub fn default() -> Self {
        SourceOverride {
            platform: Platform::default(),
            shell_command: None,
            check_command: None,
            install_command: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PackageSource {
    /// The name of the package manager.
    pub name: KnownSources,
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
    pub prepend_to_package_name: Option<String>,

    /// Override the commands per platform.
    pub overrides: Option<Vec<SourceOverride>>,
}

impl PackageSource {
    pub fn name_str(&self) -> String {
        self.name.to_string()
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
    pub fn get_override_for_current_platform(&self) -> Option<SourceOverride> {
        let current = Platform::current();
        match &self.overrides {
            Some(overrides) => overrides.iter().find(|&o| o.platform == current).cloned(),
            None => None,
        }
    }

    /// Returns the configured shell command, taking into account any platform overrides.
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

    pub fn install_packages_command(&self, packages: Vec<String>) -> String {
        format!("{} {}", self.install_command, packages.join(" "))
    }

    /// Returns the configured check command, taking into account any platform overrides.
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

    pub fn packages(&self) -> Vec<String> {
        let pkg_list = self.exec_check();
        let lines = pkg_list.lines();
        let packages: Vec<String> = lines.map(|s| self.adjust_package_name(s)).collect();
        debug!("{} - {} packages installed", self.name, packages.len());
        trace!("{:?}", packages);
        packages
    }


    pub fn adjust_package_name(&self, pkg: &str) -> String {
        match &self.prepend_to_package_name {
            Some(pre) => format!("{}{}", pre, pkg),
            None => pkg.to_string(),
        }
    }


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

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

use crate::SantaConfig;
use std::collections::{HashMap, HashSet};

// use cached::proc_macro::cached;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize, __private::de::IdentifierDeserializer};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::data::{KnownElves, PackageData, Platform, SantaData};

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
    pub fn check(&self, elf: &Elf, pkg: &str) -> bool {
        match self.cache.get(&elf.name_str()) {
            Some(pkgs) => pkgs.contains(&pkg.to_string()),
            _ => {
                debug!("No package cache for {}", elf);
                false
            }
        }
    }

    pub fn cache_for(&mut self, elf: &Elf) {
        info!("Caching data for {}", elf);
        let pkgs = elf.packages();
        self.cache.insert(elf.name_str(), pkgs.clone());
    }

    /// Returns all packages for an Elf. This will call the Elf's check_command and populate the cache if needed.
    /// If the Elf can't be found, or the cache population fails, then None will be returned.
    pub fn packages_for(cache: &mut PackageCache, elf: &Elf) -> Option<Vec<String>> {
        let c = cache.clone();
        match c.cache.get(&elf.name_str()) {
            Some(pkgs) => {
                trace!("Cache hit");
                Some(pkgs.to_vec())
            }
            None => {
                debug!("Cache miss, filling cache for {}", elf.name);
                let pkgs = elf.packages();
                cache.cache_for(elf);
                Some(pkgs)
                // None
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ElfOverride {
    platform: Platform,
    pub shell_command: Option<String>,
    pub install_command: Option<String>,
    pub check_command: Option<String>,
}

impl ElfOverride {
    pub fn default() -> Self {
        ElfOverride {
            platform: Platform::default(),
            shell_command: None,
            check_command: None,
            install_command: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Elf {
    /// The name of the package manager.
    pub name: KnownElves,
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
    /// A string to prepend to every package name for this elf.
    pub prepend_to_package_name: Option<String>,

    /// Override the commands per platform.
    pub overrides: Option<Vec<ElfOverride>>,
    // #[serde(skip)]
    // pub _packages: Vec<String>,

    // #[serde(skip)]
    // pub _checked: bool,
}

impl Elf {
    pub fn name_str(&self) -> String {
        self.name.to_string()
    }

    // #[cfg(target_os = "windows")]
    fn exec_check(&self) -> String {
        let check = self.check_command();
        let ex: Exec;

        debug!("Running shell command: {}", check);

        if MACHINE_KIND != "windows" {
            ex = Exec::shell(check);
        } else {
            ex = Exec::cmd("pwsh.exe").args(&[
                "-NonInteractive",
                "-NoLogo",
                "-NoProfile",
                "-Command",
                &check,
            ]);
        }
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

    pub fn exec_install(&self, config: &SantaConfig, data: &SantaData, packages: Vec<String>) {
        // let pkgs: Vec<String> = config.clone().groups(data).keys().map(|i| i.to_string()).collect();
        // for (k, v) in config.groups(data) {
        //     println!("To install missing {} packages, run:", self);
        //     println!("{} {}\n", self.install_command, pkgs.join(" "));
        // }

        if packages.len() != 0 {
            let renamed: Vec<String> = packages.iter().map(|p| data.name_for(p, self)).collect();
            println!("To install missing {} packages, run:", self);
            println!("{} {}\n", self.install_command, renamed.join(" "));
        } else {
            info!("No missing packages for {}", self);
        }
    }

    /// Returns an override for the current platform, if defined.
    pub fn get_override_for_current_platform(&self) -> Option<ElfOverride> {
        let current = Platform::current();
        match &self.overrides {
            Some(overrides) => match overrides.into_iter().find(|&o| o.platform == current) {
                Some(ov) => Some(ov.clone()),
                None => None,
            },
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

    // pub fn packages_to_install(&self, cache: &PackageCache) -> Vec<String> {
    //     self.packages()
    //         .clone()
    //         .iter()
    //         .filter(|p| self.package_is_installed(p.to_string(), cache))
    //         .map(|s| s.to_string())
    //         .collect()
    // }

    pub fn adjust_package_name(&self, pkg: &str) -> String {
        match &self.prepend_to_package_name {
            Some(pre) => format!("{}{}", pre, pkg),
            None => pkg.to_string(),
        }
    }

    // pub fn package_is_installed(&self, pkg: String, cache: &PackageCache) -> bool {
    //     self.packages().contains(&pkg)
    // }

    pub fn table(
        &self,
        pkgs: &Vec<String>,
        cache: &PackageCache,
        include_installed: bool,
    ) -> Table {
        let mut table = Table::new("{:<} {:<}");
        for pkg in pkgs {
            let installed = cache.check(&self, &pkg);
            let emoji = if installed { "✅" } else { "❌" };

            if !installed || (installed && include_installed) {
                table.add_row(Row::new().with_cell(emoji).with_cell(pkg));
            }
        }
        table
    }
}

impl std::fmt::Display for Elf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

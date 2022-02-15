use std::collections::{HashMap, HashSet};

use cached::proc_macro::cached;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize, __private::de::IdentifierDeserializer};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::data::{KnownElves, PackageData, Platform, SantaData};

pub mod traits;

pub struct PackageCache {
    pub cache: HashMap<String, Vec<String>>,
}

impl PackageCache {
    pub fn new() -> Self {
        let map: HashMap<String, Vec<String>> = HashMap::new();
        PackageCache { cache: map }
    }

    pub fn check(&self, elf: &str, pkg: &str) -> bool {
        match self.cache.get(elf) {
            Some(pkgs) => pkgs.contains(&pkg.to_string()),
            _ => {
                error!("No package cache for {}", elf);
                false
            }
        }
    }

    pub fn packages_for(&self, elf: &str) -> Option<&Vec<String>> {
        self.cache.get(elf)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Elf {
    /// The name of the package manager.
    pub name: String,
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

    #[serde(skip)]
    pub _packages: Vec<String>,

    #[serde(skip)]
    pub _checked: bool,
}

impl Elf {
    fn exec_check(&self) -> String {
        // let shell = self.shell_command();
        let check = self.check_command();
        debug!("Running shell command: {}", check);

        let command = check;
        match Exec::shell(command).capture() {
            Ok(data) => {
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                error!("{}", e);
                return "".to_string();
            }
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
                info!("Override found: {:?}", ov);
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
        trace!("{} - {} packages", self.name, packages.len());
        packages
    }

    pub fn adjust_package_name(&self, pkg: &str) -> String {
      match &self.prepend_to_package_name {
        Some(pre) => format!("{}{}", pre, pkg),
        None => pkg.to_string()
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
            let installed = cache.check(&self.name, &pkg);
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
        // let mut table = Table::new("{:<}{:<}");
        // table.add_heading(format!("{} {}", self.emoji, self.name));
        // write!(f, "{}", table)
        write!(f, "{} {}", self.emoji, self.name)
    }
}

impl traits::InstallCapable for Elf {
    fn install_packages(&self, pkg: &PackageData) {
        println!("Not Yet Implemented");
    }
}

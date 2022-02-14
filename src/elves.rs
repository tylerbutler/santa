use std::collections::{HashMap, HashSet};

use cached::proc_macro::cached;
use log::{debug, error, info};
use serde::{Deserialize, Serialize, __private::de::IdentifierDeserializer};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::data::{KnownElves, PackageData, Platform, SantaConfig, SantaData};

// use self::traits::Package;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Elf {
    pub name: String,
    emoji: String,
    pub shell_command: String,
    pub install_command: String,
    pub check_command: String,
    pub overrides: Option<Vec<ElfOverride>>,

    #[serde(skip)]
    pub _packages: Vec<String>,

    #[serde(skip)]
    pub _checked: bool,
    // #[serde(skip)]
    // pub configured_packages: Vec<String>,
}

impl Elf {
    fn exec_check(&self) -> String {
        debug!(
            "Running shell command: {} {}",
            self.shell_command, self.check_command
        );
        let command = [self.shell_command.clone(), self.check_command.clone()].join(" ");
        match Exec::shell(command).capture() {
            Ok(data) => {
                // self.set_checked();
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                // self.set_checked();
                error!("{}", e);
                return "".to_string();
            }
        }
    }

    pub fn packages(&self) -> Vec<String> {
        let pkg_list = self.exec_check();
        let lines = pkg_list.lines();
        // let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        info!("{} - {} packages", self.name, packages.len());
        packages
    }

    pub fn table(
      &self,
      // groups: &HashMap<KnownElves, Vec<String>>,
      pkgs: &Vec<String>,
      cache: &PackageCache,
      include_installed: bool,
  ) -> Table {
      let mut table = Table::new("{:<} {:<}");
      for pkg in pkgs {
          let owned_package = pkg.to_owned();
          let checked = cache.check(&self.name, &pkg);
          let add = !checked || (checked && include_installed);
          let emoji = if checked { "✅" } else { "❌" };

          if add {
              table.add_row(Row::new().with_cell(emoji).with_cell(pkg));
          }
      }
      table
  }

}

impl std::fmt::Display for Elf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new("{:<}{:<}");
        table.add_heading(format!("{} {}", self.emoji, self.name));
        write!(f, "{}", table)
    }
}

impl traits::InstallCapable for Elf {
    fn install_packages(&self, pkg: &PackageData) {
        println!("Not Yet Implemented");
    }
}

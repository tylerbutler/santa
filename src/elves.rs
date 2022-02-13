use std::collections::{HashMap, HashSet};

use cached::proc_macro::cached;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::data::{PackageData, SantaConfig};

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

    pub fn foo() -> Vec<String> {
        Vec::new()
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

    // pub fn packages(&self, elf: &str) -> Vec<String> {
    //     match self.cache.get(elf) {
    //         Some(pkgs) => pkgs,
    //         _ => {
    //             error!("Nothing in the cache for {}", elf);
    //             let v: Vec<String> = Vec::new();
    //             return &v;
    //         }
    //     }
    // }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Elf {
    pub name: String,
    emoji: String,
    pub shell_command: String,
    pub install_command: String,
    pub check_comand: String,

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
            self.shell_command, self.check_comand
        );
        let command = [self.shell_command.clone(), self.check_comand.clone()].join(" ");
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
        debug!("{} - {} packages", self.name, packages.len());
        packages
    }

    // pub fn cache_package_list(&mut self) {
    //   debug!{"Caching package list."};
    //     self._checked = true;
    //     self._packages = packages(self);
    // }
}

fn packages(elf: &Elf) -> Vec<String> {
    if elf._checked {
        debug!("Returning cached package list.");
        return elf._packages.to_owned();
        // return self._packages;
    } else {
        let pkg_list = elf.exec_check();
        let lines = pkg_list.lines();
        // let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        // Vec::new()
        packages
    }
}

pub fn table(mut elf: &Elf, config: &SantaConfig, cache: &PackageCache) -> Table {
    let mut table = Table::new("{:<}{:<}");
    for pkg in &config.packages {
        let owned_package = pkg.to_owned();
        table.add_row(
            Row::new()
                .with_cell(pkg)
                .with_cell(if cache.check(&elf.name, &pkg) {
                    "Y"
                } else {
                    "N"
                }),
        );
    }
    table
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

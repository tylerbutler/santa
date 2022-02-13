use std::collections::HashSet;

use cached::proc_macro::cached;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::data::{PackageData, SantaConfig};

// use self::traits::Package;

pub mod traits;

// pub fn all_elves() -> Vec<Elf> {
//     let mut vec: Vec<Elf> = Vec::new();
//     let brew = Elf {
//         name: "brew",
//         emoji: "🍺",
//         shell_command: "brew",
//         install_command: "install",
//         check_comand: "leaves --installed-on-request",
//         configured_packages: Vec::new(),
//     };
//     vec.push(brew);
//     return vec;
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct Elf {
    name: String,
    emoji: String,
    shell_command: String,
    install_command: String,
    check_comand: String,

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

    fn check(&self, pkg: &str) -> bool {
        packages(self).contains(&pkg.to_string())
    }

    pub fn cache_package_list(&mut self) {
        self._checked = true;
        self._packages = packages(self);
    }
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

pub fn table(mut elf: &Elf, config: &SantaConfig) -> Table {
    let mut table = Table::new("{:<}{:<}");
    for pkg in &config.packages {
        let owned_package = pkg.to_owned();
        table.add_row(
            Row::new()
                .with_cell(pkg)
                .with_cell(if elf.check(&pkg) { "Y" } else { "N" }),
        );
    }
    table
}

// impl traits::Elf for ElfData {}

// impl Printable for Elf {
//     fn title(&self) -> String {
//         return [self.emoji, self.name].join(" ");
//     }

//     fn print_status(&self) {
//         println!("{}", self.title());
//         self.list_packages();
//     }
// }

impl std::fmt::Display for Elf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new("{:<}{:<}");
        table.add_heading(format!("{} {}", self.emoji, self.name));
        write!(f, "{}", table)
    }
}

// impl traits::HasPackages for Elf {
//     fn packages(&self) -> Vec<String> {
//         if self._checked {
//             debug!("Returning cached package list.");
//             return self._packages.to_owned();
//             // return self._packages;
//         } else {
//             let pkg_list = self.exec_check();
//             let lines = pkg_list.lines();
//             // let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
//             let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
//             self._packages = packages.to_owned();
//             // Vec::new()
//             packages
//         }
//     }

//     fn check(&mut self, pkg: &str) -> bool {
//       HasPackages::packages(self).contains(&pkg.to_string())
//     }
// }

impl traits::InstallCapable for Elf {
    fn install_packages(&self, pkg: &PackageData) {
        println!("Not Yet Implemented");
    }
}

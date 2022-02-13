use std::collections::HashSet;

use cached::proc_macro::cached;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use subprocess::Exec;
use tabular::{Row, Table};

use crate::{data::{PackageData, SantaConfig}, elves::traits::CheckAndListCapable};

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
    pub configured_packages: Vec<String>,
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
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                error!("{}", e);
                return "".to_string();
            }
        }
    }

    pub fn table(&self, config: &SantaConfig) {
      let mut table = Table::new("{:<}{:<}");
      for pkg in &config.packages {
        let owned_package = pkg.to_owned();
        table.add_row(Row::new().with_cell(pkg).with_cell(if self.check(&pkg) {
            "Y"
        } else {
            "N"
        }));
    }

    }
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

impl traits::CheckAndListCapable for Elf {
    fn packages(&self) -> Vec<String> {
        let pkg_list = self.exec_check();
        let lines = pkg_list.lines();
        let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        // Vec::new()
        packages
    }
}

impl traits::InstallCapable for Elf {
    fn install_packages(&self, pkg: &PackageData) {
        println!("Not Yet Implemented");
    }
}

use std::collections::HashSet;

use log::{debug, error};
use serde::{Serialize, Deserialize};
use subprocess::Exec;

use crate::elves::traits::CheckAndListCapable;

use self::traits::Package;

pub mod traits;

pub fn all_elves<'a>() -> Vec<Elf<'a>> {
    let mut vec: Vec<Elf> = Vec::new();
    let brew = Elf {
        name: "brew",
        emoji: "🍺",
        shell_command: "brew",
        install_command: "install",
        check_comand: "leaves --installed-on-request",
    };
    vec.push(brew);
    return vec;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Elf<'a> {
    name: &'a str,
    emoji: &'a str,
    shell_command: &'a str,
    install_command: &'a str,
    check_comand: &'a str,
}

impl<'a> Elf<'a> {
    fn exec_check(&self) -> String {
        debug!(
            "Running shell command: {} {}",
            self.shell_command, self.check_comand
        );
        let command = [self.shell_command, self.check_comand].join(" ");
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
}

// impl<'a> traits::Elf for ElfData<'a> {}

// impl<'a> Printable for Elf<'a> {
//     fn title(&self) -> String {
//         return [self.emoji, self.name].join(" ");
//     }

//     fn print_status(&self) {
//         println!("{}", self.title());
//         self.list_packages();
//     }
// }

impl<'a> std::fmt::Display for Elf<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ELF: {} {}\n{:?}", self.emoji, self.name, self.packages())
    }
}

impl<'a> traits::CheckAndListCapable for Elf<'a> {
    fn packages(&self) -> Vec<String> {
        let pkg_list = self.exec_check();
        let lines = pkg_list.lines();
        let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
        // Vec::new()
        packages

    }
}

impl<'a> traits::InstallCapable for Elf<'a> {
    fn install_packages(&self, pkg: Box<dyn Package>) {
        println!("Not Yet Implemented");
    }
}

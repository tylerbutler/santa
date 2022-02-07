use std::collections::HashSet;

use log::{debug, error};
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

impl<'a> traits::Printable for Elf<'a> {
    fn title(&self) -> String {
        return [self.emoji, self.name].join(" ");
    }

    fn print_status(&self) {
        println!("{}", self.title());
        self.list_packages();
    }
}

impl<'a> traits::CheckAndListCapable for Elf<'a> {
    fn list_packages(&self) {
        let pkg_list = self.exec_check();

        let lines = pkg_list.lines();
        for line in lines {
            println!("{}", line);
        }

        // let mut pkgs: HashSet<String> = HashSet::new();
        // for line in lines {
        //     pkgs.insert(String::from(line));
        // }
    }
}

impl<'a> traits::InstallCapable for Elf<'a> {
    fn install_packages(&self, pkg: Box<dyn Package>) {
        println!("Not Yet Implemented");
    }
}

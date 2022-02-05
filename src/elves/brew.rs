use std::fmt;

use anyhow::bail;
use clap::{AppSettings, Parser, Subcommand};
use console::style;
use std::collections::HashSet;
use std::path::PathBuf;
use subprocess::*;

use crate::elves::traits;

struct BrewStruct {
    pub installed: HashSet<String>,
}

impl BrewStruct {
    fn list_packages() -> String {
        match Exec::shell("brew leaves --installed-on-request").capture() {
            Ok(data) => {
                let val = data.stdout_str();
                return val;
            }
            Err(e) => {
                // TODO
                return "".to_string();
            }
        }
    }

    fn test<T: AsRef<str>>(inp: &[T]) {
        for x in inp {
            print!("{} ", x.as_ref())
        }
        println!("");
    }
}

impl traits::Elf for BrewStruct {
    fn ListPackages(&mut self) {
        let pkg_list = BrewStruct::list_packages();
        let lines = pkg_list.lines();
        let mut pkgs: HashSet<String> = HashSet::new();
        // self.installed = pkgs.to_owned();
        for line in lines {
            self.installed.insert(String::from(line));
        }
        // self.installed = pkgs;
        // for pkg in split {
        //     self.installed.insert(pkg.to_string());
        // }
    }
}

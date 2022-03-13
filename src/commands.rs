use crate::data::ElfList;
use crate::data::KnownElves;
use crate::elves::Elf;
use crate::traits::Exportable;
use crate::{configuration::SantaConfig, elves::PackageCache};
use std::collections::HashSet;
use std::{collections::HashMap, fmt::format};

use log::{debug, info, trace, warn};

use crate::data::SantaData;

pub fn status_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache, all: &bool) {
    // filter elves to those enabled in the config
    let elves: Vec<Elf> = data
        .elves
        .clone()
        .into_iter()
        .filter(|elf| config.clone().is_elf_enabled(&elf.name_str()))
        .collect();
    let serialized = serde_yaml::to_string(&elves).unwrap();

    for elf in &elves {
        debug!("Stats for {}", elf.name);
        let pkgs = cache.cache.get(&elf.name_str());

        match pkgs {
            Some(y) => {
                debug!("Cache hit");
            }
            None => {
                debug!("Cache miss, filling cache for {}", elf.name);
                let pkgs = elf.packages();
                cache.cache.insert(elf.name_str(), pkgs);
            }
        }
    }
    for elf in &elves {
        let groups = config.clone().groups(data);
        for (key, pkgs) in groups {
            if elf.name == key {
                let pkg_count = pkgs.len();
                let table = format!("{}", elf.table(&pkgs, &cache, *all).to_string());
                println!("{} ({} packages total)", elf, pkg_count);
                println!("{}", table);
                break;
            }
        }
    }
}

pub fn config_command(config: &SantaConfig, data: &SantaData, packages: bool, builtin: bool) {
    if !builtin {
        println!("{}", config.export());
    } else {
        if packages {
            println!("{}", data.export());
        } else {
            println!("{}", data.elves.export())
        }
    }
}

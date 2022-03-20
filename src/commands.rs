use crate::data::ElfList;
use crate::data::KnownElves;
use crate::elves::Elf;
use crate::traits::Exportable;
use crate::{configuration::SantaConfig, elves::PackageCache};
use std::collections::HashSet;
use std::{collections::HashMap, fmt::format};

use log::{debug, error, info, trace, warn};

use crate::data::SantaData;

pub fn status_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache, all: &bool) {
    // filter elves to those enabled in the config
    let elves: ElfList = data
        .elves
        .clone()
        .into_iter()
        .filter(|elf| config.clone().is_elf_enabled(&elf))
        .collect();
    // let serialized = serde_yaml::to_string(&elves).unwrap();

    for elf in &elves {
        cache.cache_for(&elf);
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

    install_command(config, data, cache);
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

pub fn install_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache) {
    // let config = config.clone();
    // filter elves to those enabled in the config
    let elves: ElfList = data
        .elves
        .clone()
        .into_iter()
        .filter(|elf| config.clone().is_elf_enabled(&elf))
        .collect();

    // for (k, v) in config.groups(&data) {
    //     error!("{} {:?}", k, v);
    // }

    for elf in &elves {
        debug!("Stats for {}", elf.name);
        cache.cache_for(&elf);
    }

    // let config = config.clone();
    for elf in &elves {
        let groups = config.clone().groups(data);
        for (key, pkgs) in groups {
            if elf.name == key {
                let pkgs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| !cache.check(&elf, p))
                    .map(|p| p.to_string())
                    .collect();
                elf.exec_install(&config, data, pkgs);
            }
        }
    }
}

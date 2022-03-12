use crate::data::ElfList;
use crate::data::KnownElves;
use crate::elves::Elf;
use crate::{configuration::SantaConfig, elves::PackageCache};
use std::collections::HashSet;
use std::{collections::HashMap, fmt::format};

use log::{debug, info, trace, warn};

use crate::data::SantaData;

pub fn status_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache, all: &bool) {
    // let mut elves: HashSet<Elf> = data.elves.into_iter().collect();
    // elves.insert();
    // let elves = &data.elves;

    // filter elves to 
    let elfs: Vec<Elf> = data
        .elves
        .clone()
        .into_iter()
        .filter(|elf| config.clone().is_elf_enabled(&elf.name_str()))
        .collect();
    let serialized = serde_yaml::to_string(&elfs).unwrap();
    trace!("{}", serialized);

    for elf in &elfs {
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
    for elf in &elfs {
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

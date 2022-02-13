use crate::elves::{table, PackageCache};
use std::{collections::HashMap, fmt::format};

use log::{debug, info, warn};

use crate::data::{SantaConfig, SantaData};

pub fn status_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache) {
    let elves = &data.elves;
    let serialized = serde_yaml::to_string(&elves).unwrap();
    println!("status-comand");
    println!("{}", serialized);

    for elf in elves {
        debug!("Stata for {}", elf.name);
        let pkgs = cache.cache.get(&elf.name);

        match pkgs {
            Some(y) => {
                debug!("Cache hit");
            }
            None => {
                debug!("Cache miss, filling cache for {}", elf.name);
                let pkgs = elf.packages();
                cache.cache.insert(elf.name.to_owned(), pkgs);
            }
        }
    }
    for elf in elves {
        // elf._packages = config.packages;
        // elf.cache_package_list();
        let table = format!("{}", table(elf, config, &cache).to_string());
        println!("{}", elf);
        println!("{}", table);
    }
}

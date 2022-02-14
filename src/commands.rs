use crate::elves::{table, PackageCache};
use std::{collections::HashMap, fmt::format};

use log::{debug, info, warn};

use crate::data::{SantaConfig, SantaData};

pub fn status_command(config: SantaConfig, data: &SantaData, mut cache: PackageCache, all: &bool) {
    let elves = &data.elves;
    let serialized = serde_yaml::to_string(&elves).unwrap();
    debug!("{}", serialized);

    for elf in elves {
        debug!("Stats for {}", elf.name);
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
        // let f = elves
        let groups = config.clone().groups(data);
        for (key, pkgs) in groups {
            if elf.name == key.to_string() {
                let pkg_count = pkgs.len();
                let table = format!(
                    "{}",
                    table(elf, &pkgs, &cache, (*all || config.log_level > 0)).to_string()
                );
                println!("{}", elf);
                println!("{}", table);
                break;
            }
            // elf._packages = config.packages;
            // elf.cache_package_list();
        }
    }
}

use crate::data::KnownSources;
use crate::data::SantaData;
use crate::data::SourceList;
use crate::sources::PackageSource;
use crate::traits::Exportable;
use crate::{configuration::SantaConfig, sources::PackageCache};
use std::collections::HashSet;
use std::{collections::HashMap, fmt::format};
use anyhow::Result;

use log::{debug, error, info, trace, warn};

use colored::*;

pub fn status_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache, all: &bool) -> Result<()> {
    // filter sources to those enabled in the config
    let sources: SourceList = data
        .sources
        .clone()
        .into_iter()
        .filter(|source| config.clone().source_is_enabled(source))
        .collect();
    // let serialized = serde_yaml::to_string(&sources).unwrap();

    for source in &sources {
        cache.cache_for(source);
    }
    for source in &sources {
        let groups = config.clone().groups(data);
        for (key, pkgs) in groups {
            if source.name == key {
                let pkg_count = pkgs.len();
                let table = format!("{}", source.table(&pkgs, &cache, *all));
                println!("{} ({} packages total)", source, pkg_count);
                println!("{}", table);
                break;
            }
        }
    }
    Ok(())
}

pub fn config_command(config: &SantaConfig, data: &SantaData, packages: bool, builtin: bool) -> Result<()> {
    if !builtin {
        println!("{}", config.export());
    } else if packages {
        println!("{}", data.export());
    } else {
        println!("{}", data.sources.export())
    }
    Ok(())
}

pub fn install_command(config: &SantaConfig, data: &SantaData, mut cache: PackageCache) -> Result<()> {
    // let config = config.clone();
    // filter sources to those enabled in the config
    let sources: SourceList = data
        .sources
        .clone()
        .into_iter()
        .filter(|source| config.clone().source_is_enabled(source))
        .collect();

    // for (k, v) in config.groups(&data) {
    //     error!("{} {:?}", k, v);
    // }

    for source in &sources {
        debug!("Stats for {}", source.name);
        cache.cache_for(source);
    }

    // let config = config.clone();
    for source in &sources {
        let groups = config.clone().groups(data);
        for (key, pkgs) in groups {
            if source.name == key {
                let pkgs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| !cache.check(source, p))
                    .map(|p| p.to_string())
                    .collect();
                source.exec_install(config, data, pkgs);
            }
        }
    }
    Ok(())
}

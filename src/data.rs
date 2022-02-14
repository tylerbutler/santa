use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

extern crate yaml_rust;
use log::{debug, error, trace, warn};
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::{elves::Elf, traits::Exportable};

// #[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
// #[serde(untagged)]
// pub enum ElfOrBool {
//   KnownElves,
//   Boolean(bool),
// }

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownElves {
    Apt,
    Aur,
    Brew,
    Cargo,
    Pacman,
    Scoop,
    #[serde(other)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OS {
    Macos,
    Linux,
    Windows,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Arch {
    X64,
    Aarch64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Distro {
    None,
    ArchLinux,
    Ubuntu,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Platform {
    os: OS,
    arch: Arch,
    distro: Distro,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    pub name: Option<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    pub elves: Option<Vec<String>>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            pre: None,
            post: None,
            elves: None,
        }
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Package {
//     pub name: String,
//     // pub data: Option<PackageData>,
//     pub elves: Option<Vec<String>>,
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct SantaData {
    pub packages: HashMap<String, HashMap<KnownElves, Option<PackageData>>>,
    // pub elf_settings: HashMap<KnownElves, PackageData>,
    pub elves: Vec<Elf>,
}

impl SantaData {
    pub fn load_from(file: &Path) -> Self {
        debug!("Loading data from: {}", file.display());
        if !file.exists() {
            error!("Can't find data file: {}", file.display());
        }
        let yaml_str = fs::read_to_string(file).unwrap();
        let data: SantaData = serde_yaml::from_str(&yaml_str).unwrap();
        data
    }

    // pub fn packages(&self) -> Vec<String> {
    //     let pkg_list = self.exec_check();
    //     let lines = pkg_list.lines();
    //     // let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
    //     let packages: Vec<String> = lines.map(|s| s.to_string()).collect();
    //     debug!("{} - {} packages", self.name, packages.len());
    //     packages
    // }
}

impl Exportable for SantaData {}

#[derive(Serialize, Deserialize, Debug)]
pub struct SantaConfig {
    pub sources: Vec<KnownElves>,
    pub packages: Vec<String>,
}

impl SantaConfig {
    pub fn load_from(file: &Path) -> Self {
        debug!("Loading config from: {}", file.display());
        let mut yaml_str: String;
        if file.exists() {
            yaml_str = fs::read_to_string(file).unwrap();
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Loading default config");
            yaml_str = fs::read_to_string("santa-config.yaml").unwrap();
        }
        let data: SantaConfig = serde_yaml::from_str(&yaml_str).unwrap();
        data
    }

    pub fn groups(mut self, data: &SantaData) -> HashMap<KnownElves, Vec<String>> {
        let configured_sources: Vec<KnownElves> = self.sources;
        // let s2 = self.sources.clone();
        let mut groups: HashMap<KnownElves, Vec<String>> = HashMap::new();
        for elf in configured_sources.clone() {
            groups.insert(elf, Vec::new());
        }

        for pkg in &self.packages {
            for elf in configured_sources.clone() {
                if data.packages.contains_key(pkg) {
                    let available_sources = data.packages.get(pkg).unwrap();

                    if available_sources.contains_key(&elf) {
                        match groups.get_mut(&elf) {
                            Some(v) => {
                              debug!("Adding {} to {} list.", pkg, elf);
                              v.push(pkg.to_string());
                            }
                            None => todo!(),
                        }
                    }
                }
            }
        }

        // for pkg in &self.packages {
        //     if data.packages.contains_key(pkg) {
        //         let available_sources = data.packages.get(pkg).unwrap();

        //         for (elf, pkgs) in map.into_iter() {
        //             if available_sources.contains_key(elf) {
        //                 let pkg_settings = available_sources.get(elf).unwrap();

        //                 match pkg_settings {
        //                     Some(settings) => {
        //                         // No custom settings
        //                         todo!();
        //                     }
        //                     None => {
        //                       let mut v: Vec<String> = Vec::new();
        //                       map.entry(elf).or_insert(&v).push(pkg.to_string());
        //                       match map.entry(elf) {
        //                         Entry::Vacant(e) => {
        //                             e.insert(&vec![pkg.to_string()]);
        //                         }
        //                         Entry::Occupied(mut e) => {
        //                             e.get_mut().push(pkg.to_string());
        //                         }
        //                     }
        //                   },
        //                 }
        //             }
        //         }
        //     }
        // }

        groups
    }
}

fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

use std::{
    collections::{HashMap, HashSet},
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
    pub elves: Vec<Elf>
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
}

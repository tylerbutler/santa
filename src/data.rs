use crate::SantaConfig;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

extern crate yaml_rust;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::{data::constants::DEFAULT_CONFIG, elves::Elf, traits::Exportable};

pub mod constants;

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownElves {
    Apt,
    Aur,
    Brew,
    Cargo,
    Pacman,
    Scoop,
    Nix,
    #[serde(other)]
    Unknown(String),
}

// impl std::convert::From<&str> for KnownElves {

// }

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum OS {
    Macos,
    Linux,
    Windows,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Arch {
    X64,
    Aarch64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Distro {
    None,
    ArchLinux,
    Ubuntu,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub os: OS,
    pub arch: Arch,
    pub distro: Option<Distro>,
}

impl Default for Platform {
    fn default() -> Self {
        Platform {
            os: OS::Linux,
            arch: Arch::X64,
            distro: None,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.distro {
            Some(distro) => {
                write!(f, "{:?} {:?} ({:?})", self.os, self.arch, distro)
            }
            None => {
                write!(f, "{:?} {:?}", self.os, self.arch)
            }
        }
    }
}

impl Platform {
    pub fn current() -> Self {
        let family = std::env::consts::FAMILY;
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let mut platform: Platform = Platform::default();

        if family == "windows" {
            platform.os = OS::Windows;
        } else {
            match os {
                "macos" | "ios" => {
                    platform.os = OS::Macos;
                }
                "windows" => {
                    // unnecessary
                    platform.os = OS::Windows;
                }
                _ => platform.os = OS::Linux,
            }
        }

        match arch {
            "x86_64" => platform.arch = Arch::X64,
            "aarch64" => platform.arch = Arch::Aarch64,
            _ => todo!(),
        }

        platform
    }
}

pub trait LoadFromFile {
    fn load_from(file: &Path) -> Self
    where
        Self: Sized,
    {
        debug!("Loading data from: {}", file.display());
        if !file.exists() {
            error!("Can't find data file: {}", file.display());
        }
        let yaml_str = fs::read_to_string(file).unwrap();
        LoadFromFile::load_from_str(&yaml_str)
    }

    fn load_from_str(yaml_str: &str) -> Self
    where
        Self: Sized;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    /// Name of the package
    pub name: Option<String>,
    /// A command to run BEFORE installing the package
    pub before: Option<String>,
    /// A command to run AFTER installing the package
    pub after: Option<String>,
    /// A string to prepend to the install string
    pub pre: Option<String>,
    /// A string to postpend to the install string
    pub post: Option<String>,
    /// Elves that can install this package
    pub elves: Option<Vec<String>>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            before: None,
            after: None,
            pre: None,
            post: None,
            elves: None,
        }
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug)]
pub type PackageDataList = HashMap<String, HashMap<KnownElves, Option<PackageData>>>;

impl LoadFromFile for PackageDataList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: PackageDataList = serde_yaml::from_str(&yaml_str).unwrap();
        data
    }
}

// pub type ElfList = HashSet<Elf>;
pub type ElfList = Vec<Elf>;

impl LoadFromFile for ElfList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: ElfList = serde_yaml::from_str(&yaml_str).unwrap();
        data
    }    
}    

// impl std::fmt::Display for ElfList {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self.distro {
//             Some(distro) => {
//                 write!(f, "{} {} ({})", self.os, self.arch, distro)
//             }
//             None => {
//                 write!(f, "{} {}", self.os, self.arch)
//             }
//         }
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaData {
    pub packages: PackageDataList,
    // pub elf_settings: HashMap<KnownElves, PackageData>,
    pub elves: ElfList,
}

impl SantaData {
    pub fn load_from_str(packages_str: &str, elves_str: &str) -> Self {
        let packages = PackageDataList::load_from_str(packages_str);
        let elves = ElfList::load_from_str(elves_str);
        SantaData { packages, elves }
    }

    pub fn update_from_config(&mut self, config: &SantaConfig) {
        match &config.elves {
            Some(elves) => {
                info!("Adding {} new elves from config. {} elves already loaded.", elves.len(), self.elves.len());
                self.elves.extend(elves.clone());
            }
            None => {}
        }
    }
}

impl Default for SantaData {
    fn default() -> Self {
        SantaData::load_from_str(constants::BUILTIN_PACKAGES, constants::BUILTIN_ELVES)
    }    
}    

impl Exportable for SantaData {}

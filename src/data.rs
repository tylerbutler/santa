pub mod constants;

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

use crate::{elves::Elf, traits::Exportable, data::constants::DEFAULT_CONFIG};

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

// impl std::str::FromStr for KnownElves {
//     type Err = ();

//     fn from_str(input: &str) -> Result<KnownElves, Self::Err> {
//         match input.to_lowercase() {
//             "apt" => Ok(KnownElves::Apt),
//             "aur" => Ok(KnownElves::Aur),
//             "brew" => Ok(KnownElves::Brew),
//             "cargo" => Ok(KnownElves::Cargo),
//             "pacman" => Ok(KnownElves::Pacman),
//             "scoop" => Ok(KnownElves::Scoop),
//             _ => Err(()),
//         }
//     }
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OS {
    Macos,
    Linux,
    Windows,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Arch {
    X64,
    Aarch64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Distro {
    None,
    ArchLinux,
    Ubuntu,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub os: OS,
    pub arch: Arch,
    pub distro: Option<Distro>,
}

impl Platform {
    pub fn default() -> Self {
        Platform {
            os: OS::Linux,
            arch: Arch::X64,
            distro: None,
        }
    }

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
                "windows" => { // unnecessary
                    platform.os = OS::Windows;
                }
                _ => {
                  platform.os = OS::Linux
                }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaData {
    pub packages: HashMap<String, HashMap<KnownElves, Option<PackageData>>>,
    // pub elf_settings: HashMap<KnownElves, PackageData>,
    pub elves: Vec<Elf>,
}

impl SantaData {
    pub fn default() -> Self {
      SantaData::load_from_str(constants::BUILTIN_DATA)
    }

    pub fn load_from(file: &Path) -> Self {
        debug!("Loading data from: {}", file.display());
        if !file.exists() {
            error!("Can't find data file: {}", file.display());
        }
        let yaml_str = fs::read_to_string(file).unwrap();
        SantaData::load_from_str(&yaml_str)
    }

    pub fn load_from_str(yaml_str: &str) -> Self {
      let data: SantaData = serde_yaml::from_str(&yaml_str).unwrap();
      data
  }
}

impl Exportable for SantaData {}

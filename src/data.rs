use crate::SantaConfig;
use std::{
    collections::HashMap,
    fs,
    path::Path,
};

use tracing::{error, info};
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use anyhow::Context;

use crate::{data::constants::DEFAULT_CONFIG, sources::PackageSource, traits::Exportable};

pub mod constants;

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownSources {
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
            _ => panic!("Unsupported architecture: {}", arch),
        }

        platform
    }
}

pub trait LoadFromFile {
    fn load_from(file: &Path) -> Self
    where
        Self: Sized,
    {
        info!("Loading data from: {}", file.display());
        if !file.exists() {
            error!("Can't find data file: {}", file.display());
        }
        let yaml_str = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))
            .expect("Failed to read data file");
        LoadFromFile::load_from_str(&yaml_str)
    }

    fn load_from_str(yaml_str: &str) -> Self
    where
        Self: Sized;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    /// Name of the package
    name: Option<String>,
    /// A command to run BEFORE installing the package
    pub before: Option<String>,
    /// A command to run AFTER installing the package
    pub after: Option<String>,
    /// A string to prepend to the install string
    pub pre: Option<String>,
    /// A string to postpend to the install string
    pub post: Option<String>,
    // Sources that can install this package
    // pub sources: Option<Vec<String>>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            before: None,
            after: None,
            pre: None,
            post: None,
            // sources: None,
        }
    }
}

/// A map of package names (strings)
pub type PackageDataList = HashMap<String, HashMap<KnownSources, Option<PackageData>>>;

impl LoadFromFile for PackageDataList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: PackageDataList = match serde_yaml::from_str(yaml_str) {
            Ok(data) => data,
            Err(e) => {
                error!("Error loading data: {}", e);
                error!("Using default data");
                PackageDataList::load_from_str(DEFAULT_CONFIG)
            }
        };
        data
    }
}

impl Exportable for PackageDataList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.keys().map(|key| key.to_string()).collect();

        serde_yaml::to_string(&list).expect("Failed to serialize list")
    }
}

pub type SourceList = Vec<PackageSource>;

impl LoadFromFile for SourceList {
    fn load_from_str(yaml_str: &str) -> Self {
        let data: SourceList = serde_yaml::from_str(yaml_str)
            .expect("Failed to deserialize source list");
        data
    }
}

impl Exportable for SourceList {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        let list: Vec<String> = self.iter().map(|source| format!("{}", source)).collect();

        serde_yaml::to_string(&list).expect("Failed to serialize source list")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaData {
    pub packages: PackageDataList,
    pub sources: SourceList,
}

impl SantaData {
    pub fn load_from_str(packages_str: &str, sources_str: &str) -> Self {
        let packages = PackageDataList::load_from_str(packages_str);
        let sources = SourceList::load_from_str(sources_str);
        SantaData { packages, sources }
    }

    // pub fn update_from_config(&mut self, config: &SantaConfig) {
    pub fn sources(&self, config: &SantaConfig) -> SourceList {
        if config.sources.is_empty() {
            self.sources.clone()
        } else {
            let mut ret: SourceList = self.sources.clone();
            ret.extend(self.sources.clone());
            ret
        }
    }

    pub fn name_for(&self, package: &str, source: &PackageSource) -> String {
        match self.packages.get(package) {
            #[allow(clippy::collapsible_match)]
            Some(sources) => match sources.get(&source.name) {
                Some(pkgs) => match pkgs {
                    Some(name) => name
                        .name
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| source.adjust_package_name(package)),
                    None => source.adjust_package_name(package),
                },
                None => source.adjust_package_name(package),
            },
            None => source.adjust_package_name(package),
        }
    }
}

impl Default for SantaData {
    fn default() -> Self {
        SantaData::load_from_str(constants::BUILTIN_PACKAGES, constants::BUILTIN_SOURCES)
    }
}

impl Exportable for SantaData {
    fn export_min(&self) -> String
    where
        Self: Serialize,
    {
        serde_yaml::to_string(&self).expect("Failed to serialize SantaData")
    }
}

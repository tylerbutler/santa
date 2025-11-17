// Core data models for Santa Package Manager

use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use std::collections::HashMap;

/// Strong type for package names to prevent mixing up with other strings
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct PackageName(pub String);

/// Strong type for source names to prevent mixing up with other strings
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct SourceName(pub String);

/// Strong type for command names to prevent command/package name confusion
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Into, Serialize, Deserialize)]
pub struct CommandName(pub String);

#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownSources {
    Apt,
    Aur,
    Brew,
    Cargo,
    Flathub,
    Nix,
    Npm,
    Pacman,
    Scoop,
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

/// Package-specific configuration data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageData {
    pub name: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            before: None,
            after: None,
            pre: None,
            post: None,
        }
    }
}

/// Map of package names to their source configurations
pub type PackageDataList = HashMap<String, HashMap<KnownSources, Option<PackageData>>>;

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

/// Well-known package manager sources that Santa can interact with.
///
/// Unknown sources are captured as strings to allow forward compatibility.
#[non_exhaustive]
#[derive(Serialize_enum_str, Deserialize_enum_str, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownSources {
    /// Debian/Ubuntu APT package manager.
    Apt,
    /// Arch User Repository.
    Aur,
    /// Homebrew (macOS and Linux).
    Brew,
    /// Rust Cargo package manager.
    Cargo,
    /// Flathub Flatpak repository.
    Flathub,
    /// Nix package manager.
    Nix,
    /// Node.js npm registry.
    Npm,
    /// Arch Linux pacman package manager.
    Pacman,
    /// Scoop package manager for Windows.
    Scoop,
    /// An unrecognized source, stored as a raw string.
    #[serde(other)]
    Unknown(String),
}

/// Supported operating systems for platform targeting.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum OS {
    /// Apple macOS.
    Macos,
    /// Linux (any distribution).
    Linux,
    /// Microsoft Windows.
    Windows,
}

/// Supported CPU architectures for platform targeting.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Arch {
    /// 64-bit x86 (AMD64/Intel 64).
    X64,
    /// 32-bit x86.
    X86,
    /// 64-bit ARM (Apple Silicon, AWS Graviton, etc.).
    Aarch64,
    /// 32-bit ARM.
    Arm,
}

/// Linux distribution identifiers for distro-specific package source selection.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Distro {
    /// No specific distribution (generic Linux or non-Linux OS).
    None,
    /// Arch Linux and derivatives.
    ArchLinux,
    /// Ubuntu and derivatives.
    Ubuntu,
}

/// A target platform defined by operating system, CPU architecture, and optional Linux distribution.
///
/// Used to determine which package sources and installation methods are applicable.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    /// The operating system.
    pub os: OS,
    /// The CPU architecture.
    pub arch: Arch,
    /// The Linux distribution, if applicable.
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

/// Per-source configuration for a single package, including install hooks and name overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageData {
    /// Source-specific package name override (e.g. a different crate name in Cargo vs Brew).
    pub name: Option<String>,
    /// Shell snippet to run before the package manager command.
    pub before: Option<String>,
    /// Shell snippet to run after the package manager command.
    pub after: Option<String>,
    /// Shell snippet to run as a pre-install step (inside the generated script).
    pub pre: Option<String>,
    /// Shell snippet to run as a post-install step (inside the generated script).
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

/// Map of package names to their per-source configurations.
///
/// The outer key is the user-facing package name. The inner map associates each
/// [`KnownSources`] with an optional [`PackageData`] override for that source.
pub type PackageDataList = HashMap<String, HashMap<KnownSources, Option<PackageData>>>;

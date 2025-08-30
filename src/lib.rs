//! Santa - A package manager meta-tool
//!
//! This library provides functionality for managing packages across different platforms
//! and package managers.

pub mod commands;
pub mod completions;
pub mod configuration;
pub mod data;
pub mod errors;
pub mod plugins;
pub mod sources;
pub mod traits;

// Re-export commonly used types
pub use configuration::SantaConfig;
pub use data::{KnownSources, PackageData, SantaData};
pub use errors::{Result, SantaError};
pub use sources::{PackageCache, PackageSource};

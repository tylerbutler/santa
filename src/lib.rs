//! Santa - A package manager meta-tool
//! 
//! This library provides functionality for managing packages across different platforms
//! and package managers.

pub mod commands;
pub mod configuration;
pub mod data;
pub mod sources;
pub mod traits;

// Re-export commonly used types
pub use data::{KnownSources, PackageData, SantaData};
pub use sources::{PackageCache, PackageSource};
pub use configuration::SantaConfig;
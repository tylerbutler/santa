//! Santa - A package manager meta-tool
//!
//! Santa is a comprehensive package manager abstraction layer that provides unified
//! interfaces across different package managers and platforms. It supports multiple
//! package sources (apt, brew, cargo, etc.) and offers features like caching,
//! configuration management, and async command execution.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use santa::{SantaConfig, SantaData, KnownSources};
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load configuration from a file
//! let config = SantaConfig::load_from(Path::new("santa.yaml"))?;
//!
//! // Create Santa data manager with default built-in data
//! let santa_data = SantaData::default();
//!
//! // Get sources for the current platform
//! let sources = santa_data.sources(&config);
//!
//! // Use specific package managers
//! for source in sources {
//!     println!("Available source: {}", source.name_str());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **Multi-platform Support**: Works across Linux, macOS, and Windows
//! - **Unified Interface**: Common API for different package managers
//! - **Async Operations**: Non-blocking package operations using tokio
//! - **Caching**: Intelligent caching of package lists and metadata
//! - **Configuration**: Flexible YAML-based configuration system
//! - **Security**: Input sanitization and shell escape protection
//! - **Hot Reload**: Configuration changes without restart
//!
//! # Architecture
//!
//! Santa is built around several core concepts:
//!
//! - [`PackageSource`]: Individual package manager implementations
//! - [`SantaConfig`]: Configuration management and validation
//! - [`SantaData`]: Platform detection and source management
//! - [`PackageCache`]: Caching layer for performance
//! - [`SantaError`]: Unified error handling
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T>`] where the error type is [`SantaError`].
//! This provides structured error information with context about what operation failed.
//!
//! ```rust,no_run
//! use santa::{SantaConfig, SantaError};
//! use std::path::Path;
//!
//! match SantaConfig::load_from(Path::new("santa.yaml")) {
//!     Ok(config) => println!("Configuration loaded successfully"),
//!     Err(e) => eprintln!("Configuration error: {}", e),
//! }
//! ```
//!
//! # Safety
//!
//! Santa takes security seriously, especially around shell command execution:
//!
//! - All user inputs are sanitized before shell execution
//! - Package names are validated against known-safe patterns
//! - Command injection protection using `shell-escape` crate
//! - No raw shell command execution from user input
//!
//! # Performance Considerations
//!
//! - Caching reduces repeated package manager queries
//! - Async operations prevent blocking on slow package managers
//! - Lazy loading of configuration and package data
//! - Memory-efficient data structures with reference counting where appropriate

pub mod commands;
pub mod completions;
pub mod configuration;
pub mod data;
pub mod errors;
pub mod migration;
pub mod plugins;
pub mod script_generator;
pub mod sources;
pub mod traits;

#[cfg(feature = "tui")]
pub mod tui;

// Re-export commonly used types
pub use configuration::SantaConfig;
pub use data::{KnownSources, PackageData, SantaData};
pub use errors::{Result, SantaError};
pub use script_generator::{ExecutionMode, ScriptFormat, ScriptGenerator};
pub use sources::{PackageCache, PackageSource};

//! Error types for the Santa package manager.
//!
//! This module provides unified error handling across all Santa operations,
//! replacing the various ad-hoc error patterns with a structured approach.

use thiserror::Error;

/// The main error type for Santa operations.
///
/// This enum covers all the major error categories that can occur during
/// package management operations, configuration loading, caching, and more.
#[derive(Debug, Error)]
pub enum SantaError {
    /// Configuration-related errors (file parsing, validation, etc.)
    #[error("Configuration error: {0}")]
    Config(#[from] anyhow::Error),

    /// Package source-related errors (installation, listing, etc.)
    #[error("Package source error: {0}")]
    PackageSource(String),

    /// Command execution failures
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// Security-related violations (malicious package names, etc.)
    #[error("Security violation: {0}")]
    Security(String),

    /// Cache operation failures
    #[error("Cache operation failed: {0}")]
    Cache(String),

    /// File I/O operation failures
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(String),

    /// Package parsing or validation errors
    #[error("Invalid package: {0}")]
    InvalidPackage(String),

    /// Concurrent access or locking errors
    #[error("Concurrency error: {0}")]
    Concurrency(String),

    /// Plugin or extension loading errors
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// Template rendering or parsing errors
    #[error("Template error: {0}")]
    Template(String),
}

/// A type alias for Results that use SantaError.
pub type Result<T> = std::result::Result<T, SantaError>;

impl SantaError {
    /// Creates a new PackageSource error with context.
    pub fn package_source<S1, S2>(source: S1, msg: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SantaError::PackageSource(format!("{}: {}", source.into(), msg.into()))
    }

    /// Creates a new CommandFailed error with context.
    pub fn command_failed<S1, S2>(cmd: S1, details: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SantaError::CommandFailed(format!("{}: {}", cmd.into(), details.into()))
    }

    /// Creates a new InvalidPackage error with context.
    pub fn invalid_package<S1, S2>(package: S1, reason: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SantaError::InvalidPackage(format!("{}: {}", package.into(), reason.into()))
    }

    /// Creates a new Plugin error with context.
    pub fn plugin<S1, S2>(plugin: S1, msg: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SantaError::Plugin(format!("{}: {}", plugin.into(), msg.into()))
    }

    /// Returns true if this error represents a security violation.
    pub fn is_security_error(&self) -> bool {
        matches!(self, SantaError::Security(_))
    }

    /// Returns true if this error represents a transient failure that might be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SantaError::Network(_) | SantaError::Io(_) | SantaError::CommandFailed(_)
        )
    }

    /// Returns an optional user-facing hint for how to resolve this error.
    pub fn hint(&self) -> Option<String> {
        match self {
            SantaError::Config(err) => {
                let msg = err.to_string();
                if msg.contains("not found") || msg.contains("No such file") {
                    Some("Run `santa init` to create a config file, or use `--config` to specify a path.".into())
                } else {
                    Some("Check your config file syntax. See `santa config` or the config guide at docs/configuration.md.".into())
                }
            }
            SantaError::PackageSource(msg) => {
                if msg.contains("not found") || msg.contains("not installed") {
                    let source = msg.split(':').next().unwrap_or("the package manager");
                    Some(format!("Is `{source}` installed? Check with `which {source}`."))
                } else {
                    Some("Run `santa sources list` to check available sources.".into())
                }
            }
            SantaError::Network(msg) => {
                if msg.contains("timeout") {
                    Some("Check your internet connection and try again.".into())
                } else {
                    Some("Check your internet connection. If this persists, try `santa sources clear` and re-run.".into())
                }
            }
            SantaError::InvalidPackage(_) => {
                Some("Run `santa sources update` to get the latest package definitions.".into())
            }
            SantaError::Security(_) => {
                Some("This package name contains suspicious characters. If this is intentional, file an issue.".into())
            }
            _ => None,
        }
    }

    /// Returns the error category as a string for logging/metrics.
    pub fn category(&self) -> &'static str {
        match self {
            SantaError::Config(_) => "config",
            SantaError::PackageSource(_) => "package_source",
            SantaError::CommandFailed(_) => "command_failed",
            SantaError::Security(_) => "security",
            SantaError::Cache(_) => "cache",
            SantaError::Io(_) => "io",
            SantaError::Network(_) => "network",
            SantaError::InvalidPackage(_) => "invalid_package",
            SantaError::Concurrency(_) => "concurrency",
            SantaError::Plugin(_) => "plugin",
            SantaError::Template(_) => "template",
        }
    }
}

// Implement conversion from common error types that we might encounter

impl From<config::ConfigError> for SantaError {
    fn from(err: config::ConfigError) -> Self {
        SantaError::Config(anyhow::Error::from(err))
    }
}

impl From<sickle::Error> for SantaError {
    fn from(err: sickle::Error) -> Self {
        SantaError::Config(anyhow::Error::from(err))
    }
}

impl<T> From<std::sync::PoisonError<T>> for SantaError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        SantaError::Concurrency(format!("Mutex poisoned: {}", err))
    }
}

impl From<tokio::task::JoinError> for SantaError {
    fn from(err: tokio::task::JoinError) -> Self {
        SantaError::Concurrency(format!("Task join error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SantaError::package_source("apt", "Installation failed");
        assert_eq!(err.category(), "package_source");
        assert!(!err.is_security_error());
    }

    #[test]
    fn test_security_error_detection() {
        let err = SantaError::Security("Malicious package name detected".to_string());
        assert!(err.is_security_error());
        assert_eq!(err.category(), "security");
    }

    #[test]
    fn test_retryable_errors() {
        let network_err = SantaError::Network("Connection timeout".to_string());
        assert!(network_err.is_retryable());

        let security_err = SantaError::Security("Invalid input".to_string());
        assert!(!security_err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = SantaError::command_failed("apt install curl", "package not found");
        let error_string = format!("{}", err);
        assert!(error_string.contains("apt install curl"));
        assert!(error_string.contains("package not found"));
    }
}

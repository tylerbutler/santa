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

    /// Command execution failures
    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    /// File I/O operation failures
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),

    /// Concurrent access or locking errors
    #[error("Concurrency error: {0}")]
    Concurrency(String),

    /// Template rendering or parsing errors
    #[error("Template error: {0}")]
    Template(String),
}

/// A type alias for Results that use SantaError.
pub type Result<T> = std::result::Result<T, SantaError>;

impl SantaError {
    /// Creates a new CommandFailed error with context.
    pub fn command_failed<S1, S2>(cmd: S1, details: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SantaError::CommandFailed(format!("{}: {}", cmd.into(), details.into()))
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
        let err = SantaError::command_failed("apt install curl", "Installation failed");
        // Verify error was created correctly
        match err {
            SantaError::CommandFailed(msg) => {
                assert!(msg.contains("apt install curl"));
                assert!(msg.contains("Installation failed"));
            }
            _ => panic!("Expected CommandFailed variant"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = SantaError::command_failed("apt install curl", "package not found");
        let error_string = format!("{}", err);
        assert!(error_string.contains("apt install curl"));
        assert!(error_string.contains("package not found"));
    }
}

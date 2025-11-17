//! Error types for CCL parsing and navigation

use std::fmt;

/// Errors that can occur during CCL parsing or Model navigation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Key not found in the model
    MissingKey(String),

    /// Expected a singleton value but found a list or map
    NotASingleton,

    /// Expected a list but found a singleton or map
    NotAList,

    /// Expected a map but found a singleton or list
    NotAMap,

    /// Failed to parse value as requested type
    ValueError(String),

    /// Invalid CCL syntax
    ParseError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingKey(key) => write!(f, "key not found: {}", key),
            Error::NotASingleton => write!(f, "expected a singleton value"),
            Error::NotAList => write!(f, "expected a list"),
            Error::NotAMap => write!(f, "expected a map/object"),
            Error::ValueError(msg) => write!(f, "value error: {}", msg),
            Error::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type for CCL operations
pub type Result<T> = std::result::Result<T, Error>;

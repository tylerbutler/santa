//! Configured reader for CCL documents
//!
//! A `CclReader` borrows a `CclObject` and carries pre-configured options
//! for typed access, so callers don't need to pass options on every call.
//!
//! # Example
//!
//! ```rust
//! # use sickle::{load, CclObject};
//! # use sickle::model::{BoolOptions, ListOptions};
//! # use sickle::reader::CclReader;
//! # fn example() -> sickle::error::Result<()> {
//! let model = load("enabled = yes\nservers =\n  = web1\n  = web2")?;
//! let reader = CclReader::new(&model)
//!     .with_bool_options(BoolOptions::new().with_lenient())
//!     .with_list_options(ListOptions::new().with_coerce());
//!
//! let enabled = reader.get_bool("enabled")?;  // uses lenient mode
//! let servers = reader.get_list("servers")?;   // uses coercion
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::model::{BoolOptions, CclObject, ListOptions};
use std::str::FromStr;

/// A configured reader over a borrowed `CclObject`.
///
/// Carries `BoolOptions` and `ListOptions` so they don't need to be
/// passed on every accessor call.
#[derive(Debug, Clone)]
pub struct CclReader<'a> {
    model: &'a CclObject,
    bool_options: BoolOptions,
    list_options: ListOptions,
}

impl<'a> CclReader<'a> {
    /// Create a reader with default options
    pub fn new(model: &'a CclObject) -> Self {
        Self {
            model,
            bool_options: BoolOptions::new(),
            list_options: ListOptions::new(),
        }
    }

    /// Set boolean parsing options
    pub fn with_bool_options(mut self, options: BoolOptions) -> Self {
        self.bool_options = options;
        self
    }

    /// Set list access options
    pub fn with_list_options(mut self, options: ListOptions) -> Self {
        self.list_options = options;
        self
    }

    /// Get the underlying model
    pub fn model(&self) -> &CclObject {
        self.model
    }

    /// Create a sub-reader for a nested key
    ///
    /// The sub-reader inherits the same options but operates on the
    /// nested `CclObject` at the given key.
    pub fn get_reader(&self, key: &str) -> Result<CclReader<'a>> {
        let child = self.model.get(key)?;
        Ok(CclReader {
            model: child,
            bool_options: self.bool_options,
            list_options: self.list_options,
        })
    }

    // ========================================================================
    // Scalar access
    // ========================================================================

    /// Get a string value by key
    pub fn get_string(&self, key: &str) -> Result<&str> {
        self.model.get_string(key)
    }

    /// Get an integer value by key
    pub fn get_int(&self, key: &str) -> Result<i64> {
        self.model.get_int(key)
    }

    /// Get a float value by key
    pub fn get_float(&self, key: &str) -> Result<f64> {
        self.model.get_float(key)
    }

    /// Get a boolean value by key (uses configured BoolOptions)
    pub fn get_bool(&self, key: &str) -> Result<bool> {
        self.model.get_bool_with_options(key, self.bool_options)
    }

    // ========================================================================
    // List access
    // ========================================================================

    /// Get a list of string values by key (uses configured ListOptions)
    pub fn get_list(&self, key: &str) -> Result<Vec<String>> {
        self.model.get_list_with_options(key, self.list_options)
    }

    /// Get a typed list by key (uses configured ListOptions)
    pub fn get_list_typed<T>(&self, key: &str) -> Result<Vec<T>>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        self.model
            .get_list_typed_with_options(key, self.list_options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(ccl: &str) -> CclObject {
        crate::load(ccl).expect("should parse")
    }

    #[test]
    fn test_reader_get_string() {
        let model = make_model("name = Alice");
        let reader = CclReader::new(&model);
        assert_eq!(reader.get_string("name").unwrap(), "Alice");
    }

    #[test]
    fn test_reader_get_int() {
        let model = make_model("port = 8080");
        let reader = CclReader::new(&model);
        assert_eq!(reader.get_int("port").unwrap(), 8080);
    }

    #[test]
    fn test_reader_get_float() {
        let model = make_model("rate = 3.14");
        let reader = CclReader::new(&model);
        let val = reader.get_float("rate").unwrap();
        assert!((val - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reader_get_bool_strict() {
        let model = make_model("enabled = true");
        let reader = CclReader::new(&model);
        assert!(reader.get_bool("enabled").unwrap());
    }

    #[test]
    fn test_reader_get_bool_strict_rejects_yes() {
        let model = make_model("enabled = yes");
        let reader = CclReader::new(&model);
        assert!(reader.get_bool("enabled").is_err());
    }

    #[test]
    fn test_reader_get_bool_lenient() {
        let model = make_model("enabled = yes");
        let reader = CclReader::new(&model).with_bool_options(BoolOptions::new().with_lenient());
        assert!(reader.get_bool("enabled").unwrap());
    }

    #[test]
    fn test_reader_get_list() {
        let model = make_model("servers =\n  = web1\n  = web2");
        let reader = CclReader::new(&model);
        assert_eq!(reader.get_list("servers").unwrap(), vec!["web1", "web2"]);
    }

    #[test]
    fn test_reader_sub_reader() {
        let model = make_model("config =\n  host = localhost\n  port = 8080");
        let reader = CclReader::new(&model).with_bool_options(BoolOptions::new().with_lenient());
        let config = reader.get_reader("config").unwrap();
        assert_eq!(config.get_string("host").unwrap(), "localhost");
        assert_eq!(config.get_int("port").unwrap(), 8080);
        // Inherits lenient bool options
        assert_eq!(config.bool_options.lenient, true);
    }

    #[test]
    fn test_reader_missing_key() {
        let model = make_model("name = Alice");
        let reader = CclReader::new(&model);
        assert!(reader.get_string("missing").is_err());
    }
}

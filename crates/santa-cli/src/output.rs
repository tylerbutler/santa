//! Output formatting utilities for Santa CLI.
//!
//! This module provides colored output helpers and formatting utilities
//! for consistent user-facing messages across the Santa CLI.
//!
//! # Examples
//!
//! ```rust,no_run
//! use santa::output::{success, error, warning, info};
//!
//! success("Package installed successfully");
//! error("Failed to install package");
//! warning("Package may not be available");
//! info("Checking package status");
//! ```

use colored::Colorize;

/// Print a success message in green with a checkmark
///
/// # Arguments
///
/// * `msg` - The message to display
///
/// # Examples
///
/// ```rust,no_run
/// use santa::output::success;
///
/// success("Package installed successfully");
/// ```
pub fn success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// Print an error message in red with an X mark
///
/// # Arguments
///
/// * `msg` - The error message to display
///
/// # Examples
///
/// ```rust,no_run
/// use santa::output::error;
///
/// error("Failed to install package");
/// ```
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print a warning message in yellow with a warning sign
///
/// # Arguments
///
/// * `msg` - The warning message to display
///
/// # Examples
///
/// ```rust,no_run
/// use santa::output::warning;
///
/// warning("Package may not be available on this platform");
/// ```
pub fn warning(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}

/// Print an info message in blue with an info icon
///
/// # Arguments
///
/// * `msg` - The informational message to display
///
/// # Examples
///
/// ```rust,no_run
/// use santa::output::info;
///
/// info("Checking package status...");
/// ```
pub fn info(msg: &str) {
    println!("{} {}", "ℹ".blue(), msg);
}

/// Format a package name with emphasis
///
/// # Arguments
///
/// * `name` - The package name to format
///
/// # Returns
///
/// A formatted string with the package name emphasized
///
/// # Examples
///
/// ```rust
/// use santa::output::package_name;
///
/// let formatted = package_name("rust-analyzer");
/// println!("Installing {}", formatted);
/// ```
pub fn package_name(name: &str) -> String {
    name.cyan().to_string()
}

/// Format a source name with emphasis
///
/// # Arguments
///
/// * `name` - The source name to format
///
/// # Returns
///
/// A formatted string with the source name emphasized
///
/// # Examples
///
/// ```rust
/// use santa::output::source_name;
///
/// let formatted = source_name("cargo");
/// println!("Using source: {}", formatted);
/// ```
pub fn source_name(name: &str) -> String {
    name.magenta().to_string()
}

/// Format a count with emphasis
///
/// # Arguments
///
/// * `count` - The count to format
///
/// # Returns
///
/// A formatted string with the count emphasized
///
/// # Examples
///
/// ```rust
/// use santa::output::count;
///
/// let formatted = count(42);
/// println!("Found {} packages", formatted);
/// ```
pub fn count(n: usize) -> String {
    n.to_string().bold().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_format() {
        let name = package_name("rust-analyzer");
        assert!(name.contains("rust-analyzer"));
    }

    #[test]
    fn test_source_name_format() {
        let name = source_name("cargo");
        assert!(name.contains("cargo"));
    }

    #[test]
    fn test_count_format() {
        let formatted = count(42);
        assert!(formatted.contains("42"));
    }
}

//! End-to-end test modules for Santa CLI
//!
//! These tests use `assert_cmd` and `predicates` to test the CLI as a black box,
//! verifying user-facing behavior and command-line interfaces.

pub mod config_tests;
pub mod error_tests;
pub mod status_tests;

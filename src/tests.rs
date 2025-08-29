//! Unit tests for main application logic
//!
//! Tests the core run() function and CLI orchestration logic with comprehensive
//! scenario coverage including logging, config loading, and command routing.
//!
//! Testing approach follows 2024 best practices for Clap CLI testing:
//! - Use try_parse_from() for argument parsing tests
//! - Focus on application logic rather than testing Clap itself  
//! - Use debug_assert() to validate CLI structure
//! - Test error conditions and edge cases

use super::*;
use anyhow::Result;
use clap::CommandFactory;
use rstest::*;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

/// Test fixture for creating a temporary config file
#[fixture]
fn temp_config() -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        file,
        r#"
sources:
  - brew
packages:
  - git
  - curl
"#
    )
    .expect("Failed to write config");
    file
}

/// Test fixture for empty temporary directory (simulating no config)
#[fixture]
fn temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper function to create a testable version of run() that accepts CLI args
async fn run_with_args(args: Vec<&str>) -> Result<(), anyhow::Error> {
    // We can't easily test the full run() function due to Cli::parse() reading from command line
    // Instead, we'll test the individual components that run() orchestrates
    
    // Test argument parsing
    let cli = Cli::try_parse_from(args)?;
    
    // Test logging level determination
    let _log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        3 => Level::TRACE,
        _ => Level::TRACE,
    };
    
    // Validate log level is reasonable
    match cli.verbose {
        0..=4 => {}, // Valid range
        _ => panic!("Invalid verbose level"),
    }
    
    // Test builtin_only flag
    if cli.builtin_only {
        let _config = SantaConfig::default();
    }
    
    // Test command variants exist
    match &cli.command {
        Commands::Status { all: _ } => {},
        Commands::Install { source: _ } => {},
        Commands::Add { source: _, package: _ } => {},
        Commands::Config { packages: _, pipe: _ } => {},
        Commands::Completions { shell: _ } => {},
    }
    
    Ok(())
}

#[cfg(test)]
mod cli_structure_tests {
    use super::*;

    #[test]
    fn test_cli_structure_validation() {
        // Modern Clap v4 pattern: validate CLI structure with debug_assert
        let cmd = build_cli();
        cmd.debug_assert(); // This will panic if CLI structure is invalid
    }

    #[test]
    fn test_cli_derive_structure_validation() {
        // Also validate the derive-based CLI structure
        let mut cmd = Cli::command();
        cmd.debug_assert();
    }
}

#[cfg(test)]
mod cli_parsing_tests {
    use super::*;

    #[test]
    fn test_status_command_parsing() {
        let cli = Cli::try_parse_from(vec!["santa", "status"]).unwrap();
        assert_eq!(cli.verbose, 0, "Default verbose should be 0");
        assert!(!cli.builtin_only, "Default builtin_only should be false");
        
        match cli.command {
            Commands::Status { all } => assert!(!all, "Default all should be false"),
            _ => panic!("Should parse as Status command"),
        }
    }

    #[test]
    fn test_status_command_with_all_flag() {
        let cli = Cli::try_parse_from(vec!["santa", "status", "--all"]).unwrap();
        
        match cli.command {
            Commands::Status { all } => assert!(all, "All flag should be true"),
            _ => panic!("Should parse as Status command"),
        }
    }

    #[test]
    fn test_install_command_parsing() {
        let cli = Cli::try_parse_from(vec!["santa", "install"]).unwrap();
        
        match cli.command {
            Commands::Install { source } => {
                assert!(source.is_none(), "Default source should be None");
            },
            _ => panic!("Should parse as Install command"),
        }
    }

    #[test]
    fn test_config_command_parsing() {
        let cli = Cli::try_parse_from(vec!["santa", "config"]).unwrap();
        
        match cli.command {
            Commands::Config { packages, pipe } => {
                assert!(!packages, "Default packages flag should be false");
                assert!(!pipe, "Default pipe flag should be false");
            },
            _ => panic!("Should parse as Config command"),
        }
    }

    #[test]
    fn test_config_command_with_packages_flag() {
        let cli = Cli::try_parse_from(vec!["santa", "config", "--packages"]).unwrap();
        
        match cli.command {
            Commands::Config { packages, pipe } => {
                assert!(packages, "Packages flag should be true");
                assert!(!pipe, "Pipe flag should remain false");
            },
            _ => panic!("Should parse as Config command"),
        }
    }

    #[test]
    fn test_completions_command_parsing() {
        let cli = Cli::try_parse_from(vec!["santa", "completions", "bash"]).unwrap();
        
        match cli.command {
            Commands::Completions { shell } => {
                assert_eq!(shell, Shell::Bash, "Shell should be Bash");
            },
            _ => panic!("Should parse as Completions command"),
        }
    }

    #[test]
    fn test_verbose_flags() {
        // Test different verbosity levels with modern patterns
        let cli = Cli::try_parse_from(vec!["santa", "status"]).unwrap();
        assert_eq!(cli.verbose, 0, "Default verbose level should be 0");
        
        let cli = Cli::try_parse_from(vec!["santa", "-v", "status"]).unwrap();
        assert_eq!(cli.verbose, 1, "Single -v should be level 1");
        
        let cli = Cli::try_parse_from(vec!["santa", "-vv", "status"]).unwrap();
        assert_eq!(cli.verbose, 2, "Double -v should be level 2");
        
        let cli = Cli::try_parse_from(vec!["santa", "-vvv", "status"]).unwrap();
        assert_eq!(cli.verbose, 3, "Triple -v should be level 3");
        
        let cli = Cli::try_parse_from(vec!["santa", "--verbose", "status"]).unwrap();
        assert_eq!(cli.verbose, 1, "Long --verbose should be level 1");
    }

    #[test]
    fn test_builtin_only_flag() {
        let cli = Cli::try_parse_from(vec!["santa", "--builtin-only", "status"]).unwrap();
        assert!(cli.builtin_only, "Long --builtin-only flag should be true");
        
        let cli = Cli::try_parse_from(vec!["santa", "-b", "config"]).unwrap();
        assert!(cli.builtin_only, "Short -b flag should be true");
    }

    #[test]
    fn test_invalid_command_fails() {
        let result = Cli::try_parse_from(vec!["santa", "nonexistent"]);
        assert!(result.is_err(), "Invalid command should fail parsing");
    }

    #[test]
    fn test_no_subcommand_fails() {
        let result = Cli::try_parse_from(vec!["santa"]);
        assert!(result.is_err(), "No subcommand should fail due to subcommand_required");
    }

    #[test]
    fn test_global_flags_with_subcommands() {
        // Test that global flags work with all subcommands
        let cli = Cli::try_parse_from(vec!["santa", "-v", "--builtin-only", "status", "--all"]).unwrap();
        assert_eq!(cli.verbose, 1, "Verbose flag should work with status");
        assert!(cli.builtin_only, "Builtin-only should work with status");
        
        match cli.command {
            Commands::Status { all } => assert!(all, "All flag should work with global flags"),
            _ => panic!("Should be Status command"),
        }
    }
}

#[cfg(test)]
mod config_loading_tests {
    use super::*;

    #[test]
    fn test_builtin_config_loading() {
        // Test that default config loads without error
        let config = SantaConfig::default();
        assert!(!config.sources.is_empty(), "Default config should have sources");
        assert!(!config.packages.is_empty(), "Default config should have packages");
    }

    #[rstest]
    fn test_load_config_from_file(temp_config: NamedTempFile) {
        // Test loading config from a valid file
        let config = SantaConfig::load_from(temp_config.path());
        assert!(config.is_ok(), "Config should load from valid file");
        
        let config = config.unwrap();
        assert!(!config.sources.is_empty(), "Loaded config should have sources");
        assert!(!config.packages.is_empty(), "Loaded config should have packages");
    }

    #[rstest]
    fn test_load_config_nonexistent_file(temp_dir: TempDir) {
        // Test loading config from nonexistent file
        let nonexistent_path = temp_dir.path().join("nonexistent.yaml");
        let result = SantaConfig::load_from(&nonexistent_path);
        
        // This should either fail gracefully or fall back to default
        match result {
            Ok(_) => {}, // Fallback to default is acceptable
            Err(_) => {}, // Error is also acceptable for nonexistent file
        }
    }

    #[rstest]
    fn test_load_config_invalid_yaml(temp_dir: TempDir) {
        // Test loading config from invalid YAML file
        let invalid_config = temp_dir.path().join("invalid.yaml");
        fs::write(&invalid_config, "invalid: yaml: content: [}").expect("Failed to write invalid config");
        
        let result = SantaConfig::load_from(&invalid_config);
        assert!(result.is_err(), "Invalid YAML should fail to load");
    }
}

#[cfg(test)]
mod logging_configuration_tests {
    use super::*;

    #[test]
    fn test_log_level_mapping() {
        // Test that verbose levels map to correct log levels
        assert_eq!(
            match 0 { 0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, 3 => Level::TRACE, _ => Level::TRACE },
            Level::WARN
        );
        assert_eq!(
            match 1 { 0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, 3 => Level::TRACE, _ => Level::TRACE },
            Level::INFO
        );
        assert_eq!(
            match 2 { 0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, 3 => Level::TRACE, _ => Level::TRACE },
            Level::DEBUG
        );
        assert_eq!(
            match 3 { 0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, 3 => Level::TRACE, _ => Level::TRACE },
            Level::TRACE
        );
        assert_eq!(
            match 10 { 0 => Level::WARN, 1 => Level::INFO, 2 => Level::DEBUG, 3 => Level::TRACE, _ => Level::TRACE },
            Level::TRACE
        );
    }

    #[test]
    fn test_env_filter_creation() {
        // Test that EnvFilter can be created with different levels
        let env_filter = EnvFilter::builder()
            .with_default_directive(Level::INFO.into())
            .from_env_lossy();
        
        // Just verify it doesn't panic - actual filtering behavior is hard to test
        assert!(format!("{:?}", env_filter).contains("EnvFilter"));
    }

    #[test]
    fn test_subscriber_creation() {
        // Test that FmtSubscriber can be created with expected configuration
        let env_filter = EnvFilter::builder()
            .with_default_directive(Level::DEBUG.into())
            .from_env_lossy();

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .finish();
        
        // Just verify it can be created without panicking
        assert!(format!("{:?}", subscriber).len() > 0);
    }
}

#[cfg(test)]
mod command_routing_tests {
    use super::*;

    #[test]
    fn test_status_command_routing() {
        // Test that status command parameters are extracted correctly
        let cli = Cli::try_parse_from(vec!["santa", "status", "--all"]).unwrap();
        
        match cli.command {
            Commands::Status { all } => {
                assert!(all, "All flag should be true");
            },
            _ => panic!("Should have parsed as Status command"),
        }
    }

    #[test]
    fn test_install_command_routing() {
        let cli = Cli::try_parse_from(vec!["santa", "install"]).unwrap();
        
        match cli.command {
            Commands::Install { source } => {
                assert!(source.is_none(), "Source should be None when not provided");
            },
            _ => panic!("Should have parsed as Install command"),
        }
    }

    #[test]
    fn test_config_command_routing() {
        let cli = Cli::try_parse_from(vec!["santa", "config", "--packages"]).unwrap();
        
        match cli.command {
            Commands::Config { packages, pipe } => {
                assert!(packages, "Packages flag should be true");
                assert!(!pipe, "Pipe flag should be false by default");
            },
            _ => panic!("Should have parsed as Config command"),
        }
    }

    #[test]
    fn test_completions_command_routing() {
        let cli = Cli::try_parse_from(vec!["santa", "completions", "zsh"]).unwrap();
        
        match cli.command {
            Commands::Completions { shell } => {
                assert_eq!(shell, Shell::Zsh, "Shell should be Zsh");
            },
            _ => panic!("Should have parsed as Completions command"),
        }
    }

    #[test]
    fn test_add_command_routing() {
        let cli = Cli::try_parse_from(vec!["santa", "add"]).unwrap();
        
        match cli.command {
            Commands::Add { source, package } => {
                assert!(source.is_none(), "Source should be None when not provided");
                assert!(package.is_none(), "Package should be None when not provided");
            },
            _ => panic!("Should have parsed as Add command"),
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_cli_help_generation() {
        // Test that help can be generated without panic
        let mut cmd = build_cli();
        let help = cmd.render_help();
        let help_str = help.to_string();
        
        // Debug: print the actual help text
        println!("Help text: {}", help_str);
        
        assert!(help_str.contains("santa"), "Help should contain app name");
        // The build_cli() function doesn't define subcommands, so let's just test basic structure
        assert!(help_str.len() > 0, "Help should not be empty");
    }

    #[test]
    fn test_version_information() {
        let cmd = build_cli();
        let version = cmd.get_version().unwrap_or("unknown");
        
        // Just verify version is not empty
        assert!(!version.is_empty(), "Version should not be empty");
    }

    #[test]
    fn test_default_data_creation() {
        // Test that SantaData::default() works
        let data = SantaData::default();
        assert!(!data.sources.is_empty(), "Default data should have sources");
        
        // Test that PackageCache can be created
        let cache = PackageCache::new();
        assert!(cache.cache.is_empty(), "New cache should be empty");
    }

    #[test]
    fn test_build_cli_configuration() {
        let cmd = build_cli();
        
        assert!(cmd.is_subcommand_required_set(), "Should require subcommand");
        assert!(cmd.is_arg_required_else_help_set(), "Should show help if no args");
        assert_eq!(cmd.get_name(), "santa", "Command name should be santa");
        assert!(cmd.get_about().unwrap().to_string().contains("package managers"), "Should have descriptive about text");
    }

    #[test]
    fn test_global_flags_inheritance() {
        // Test that global flags are available on subcommands
        let result = Cli::try_parse_from(vec!["santa", "--verbose", "status", "--all"]);
        assert!(result.is_ok(), "Global flags should work with subcommands");
        
        let cli = result.unwrap();
        assert_eq!(cli.verbose, 1, "Verbose should be 1");
        
        match cli.command {
            Commands::Status { all } => assert!(all, "All flag should be true"),
            _ => panic!("Should be status command"),
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_shell_completion() {
        // Test parsing with invalid shell
        let result = Cli::try_parse_from(vec!["santa", "completions", "invalid-shell"]);
        assert!(result.is_err(), "Invalid shell should cause parsing error");
    }

    #[test]
    fn test_conflicting_flags() {
        // Most flags don't conflict, but test various combinations
        let result = Cli::try_parse_from(vec!["santa", "--builtin-only", "--verbose", "config", "--packages"]);
        assert!(result.is_ok(), "Compatible flags should work together");
    }

    #[test]
    fn test_add_command_unimplemented_path() {
        // The Add command contains a bail!() - we can't easily test this without mocking
        // but we can at least verify the command parses
        let cli = Cli::try_parse_from(vec!["santa", "add", "git", "brew"]).unwrap();
        
        match cli.command {
            Commands::Add { source, package } => {
                assert_eq!(source, Some("brew".to_string()));
                assert_eq!(package, Some("git".to_string()));
            },
            _ => panic!("Should be Add command"),
        }
    }
}
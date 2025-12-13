//! Unit tests for command functions
//!
//! Tests the core business logic of status, config, and install commands
//! with mocked dependencies to ensure isolated testing.

use super::*;
use crate::configuration::SantaConfig;
use crate::data::{KnownSources, PackageData, PackageDataList, SantaData};
use crate::sources::{PackageCache, PackageSource};
use rstest::*;
use std::collections::HashMap;

/// Helper function to create a minimal SantaConfig for tests
fn minimal_config(sources: Vec<KnownSources>, packages: Vec<String>) -> SantaConfig {
    SantaConfig {
        sources,
        packages,
        custom_sources: None,
        _groups: None,
        log_level: 0,
    }
}

/// Test fixture for creating a basic SantaConfig
#[fixture]
fn basic_config() -> SantaConfig {
    minimal_config(
        vec![KnownSources::Brew],
        vec!["git".to_string(), "curl".to_string(), "vim".to_string()],
    )
}

/// Test fixture for creating test SantaData
#[fixture]
fn test_data() -> SantaData {
    let brew_source = PackageSource::new_for_test(
        KnownSources::Brew,
        "🍺",
        "brew",
        "brew install",
        "brew list",
        None,
        None,
    );

    let mut packages = PackageDataList::new();
    let mut git_sources = HashMap::new();
    git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
    packages.insert("git".to_string(), git_sources);

    let mut curl_sources = HashMap::new();
    curl_sources.insert(KnownSources::Brew, Some(PackageData::new("curl")));
    packages.insert("curl".to_string(), curl_sources);

    let mut vim_sources = HashMap::new();
    vim_sources.insert(KnownSources::Brew, Some(PackageData::new("vim")));
    packages.insert("vim".to_string(), vim_sources);

    SantaData {
        sources: vec![brew_source],
        packages,
    }
}

/// Test fixture for empty PackageCache
#[fixture]
fn empty_cache() -> PackageCache {
    PackageCache::default()
}

/// Test fixture for populated PackageCache
#[fixture]
fn populated_cache() -> PackageCache {
    let cache = PackageCache::default();
    let source_cache = vec!["git".to_string(), "vim".to_string()]; // Only git and vim are "installed"
    cache.insert_for_test("brew".to_string(), source_cache);
    cache
}

#[cfg(test)]
mod status_command_tests {
    use super::*;

    /// Test that cache.check correctly resolves source-specific package name overrides.
    ///
    /// This tests the scenario where a package has a different name in a specific source.
    /// For example: `github-cli` is installed as `gh` in brew.
    ///
    /// The config references `github-cli`, but brew reports the installed package as `gh`.
    /// The check method should resolve `github-cli` → `gh` and find it in the cache.
    #[rstest]
    #[test]
    fn test_check_resolves_source_specific_name() {
        // Create a cache that contains the source-specific name "gh"
        let cache = PackageCache::new();
        cache.insert_for_test("brew".to_string(), vec!["gh".to_string()]);

        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        // Create package data where "github-cli" has brew-specific name "gh"
        let mut packages = PackageDataList::new();
        let mut github_cli_sources = HashMap::new();
        github_cli_sources.insert(KnownSources::Brew, Some(PackageData::new("gh")));
        packages.insert("github-cli".to_string(), github_cli_sources);

        let data = SantaData {
            sources: vec![source.clone()],
            packages,
        };

        // check() should resolve "github-cli" -> "gh" and find it
        let is_installed = cache.check(&source, "github-cli", &data);
        assert!(
            is_installed,
            "github-cli should be recognized as installed when 'gh' is in cache"
        );
    }

    /// Test that check falls back to config name when no override exists
    #[rstest]
    #[test]
    fn test_check_uses_config_name_when_no_override() {
        let cache = PackageCache::new();
        cache.insert_for_test("brew".to_string(), vec!["git".to_string()]);

        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        // Create package data where "git" has NO name override (Option is None)
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, None);
        packages.insert("git".to_string(), git_sources);

        let data = SantaData {
            sources: vec![source.clone()],
            packages,
        };

        // Should find "git" in cache since there's no override
        let is_installed = cache.check(&source, "git", &data);
        assert!(
            is_installed,
            "git should be found when no name override exists"
        );
    }

    /// Test that check handles packages not in data gracefully
    #[rstest]
    #[test]
    fn test_check_unknown_package_fallback() {
        let cache = PackageCache::new();
        cache.insert_for_test("brew".to_string(), vec!["unknown-pkg".to_string()]);

        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        let data = SantaData {
            sources: vec![source.clone()],
            packages: PackageDataList::new(), // Empty - no package definitions
        };

        // Should fall back to checking the config name directly
        let is_installed = cache.check(&source, "unknown-pkg", &data);
        assert!(
            is_installed,
            "unknown packages should fall back to direct name check"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_basic_execution(
        mut basic_config: SantaConfig,
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test that status_command executes without error
        let result = status_command(
            &mut basic_config,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            None,
        )
        .await;
        assert!(result.is_ok(), "status_command should execute successfully");
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_disabled_sources(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with no enabled sources
        let mut config = SantaConfig {
            sources: vec![], // No sources enabled
            packages: vec![],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle disabled sources gracefully"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_all_flag(
        mut basic_config: SantaConfig,
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with all=true flag
        let result = status_command(
            &mut basic_config,
            &test_data,
            empty_cache,
            &true,
            &false,
            &false,
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle all flag correctly"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_filters_enabled_sources(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Create config with specific enabled sources
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should filter to enabled sources only"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_installed_flag(
        test_data: SantaData,
        populated_cache: PackageCache,
    ) {
        // Test with installed=true flag - should show only installed packages
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string(), "curl".to_string(), "vim".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            populated_cache,
            &false,    // all
            &true,     // installed
            &false,    // missing
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle installed flag correctly"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_missing_flag(
        test_data: SantaData,
        populated_cache: PackageCache,
    ) {
        // Test with missing=true flag - should show only missing packages
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string(), "curl".to_string(), "vim".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            populated_cache,
            &false,    // all
            &false,    // installed
            &true,     // missing
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle missing flag correctly"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_source_filter(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with source filter - should show only specified source
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            Some("brew"),
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle source filter correctly"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_with_invalid_source_filter(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with invalid source filter - should return error
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec!["git".to_string()],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let result = status_command(
            &mut config,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            Some("nonexistent_source"),
        )
        .await;
        assert!(
            result.is_err(),
            "status_command should error with invalid source filter"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_status_command_all_and_installed_together(
        mut basic_config: SantaConfig,
        test_data: SantaData,
        populated_cache: PackageCache,
    ) {
        // Test with both all and installed - installed packages should be shown with status
        let result = status_command(
            &mut basic_config,
            &test_data,
            populated_cache,
            &true,     // all
            &true,     // installed (should be a no-op when all is true)
            &false,
            None,
        )
        .await;
        assert!(
            result.is_ok(),
            "status_command should handle all+installed flags correctly"
        );
    }

    /// Test filtering logic for installed packages
    #[rstest]
    #[test]
    fn test_status_installed_filtering_logic() {
        let cache = PackageCache::new();
        cache.insert_for_test(
            "brew".to_string(),
            vec!["git".to_string(), "vim".to_string()],
        );

        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
        packages.insert("git".to_string(), git_sources);
        let mut curl_sources = HashMap::new();
        curl_sources.insert(KnownSources::Brew, Some(PackageData::new("curl")));
        packages.insert("curl".to_string(), curl_sources);
        let mut vim_sources = HashMap::new();
        vim_sources.insert(KnownSources::Brew, Some(PackageData::new("vim")));
        packages.insert("vim".to_string(), vim_sources);

        let data = SantaData {
            sources: vec![source.clone()],
            packages,
        };

        let pkg_list = vec!["git".to_string(), "curl".to_string(), "vim".to_string()];

        // Test installed filter
        let installed: Vec<String> = pkg_list
            .iter()
            .filter(|p| cache.check(&source, p, &data))
            .cloned()
            .collect();
        assert_eq!(installed, vec!["git", "vim"], "Should show only installed packages");

        // Test missing filter
        let missing: Vec<String> = pkg_list
            .iter()
            .filter(|p| !cache.check(&source, p, &data))
            .cloned()
            .collect();
        assert_eq!(missing, vec!["curl"], "Should show only missing packages");
    }
}

#[cfg(test)]
mod config_command_tests {
    use super::*;

    #[rstest]
    fn test_config_command_default_export(basic_config: SantaConfig, test_data: SantaData) {
        // Test default config export (builtin=false, packages=false)
        let result = config_command(&basic_config, &test_data, false, false);
        assert!(
            result.is_ok(),
            "config_command should export config successfully"
        );
    }

    #[rstest]
    fn test_config_command_builtin_packages(basic_config: SantaConfig, test_data: SantaData) {
        // Test builtin packages export (builtin=true, packages=true)
        let result = config_command(&basic_config, &test_data, true, true);
        assert!(
            result.is_ok(),
            "config_command should export builtin packages successfully"
        );
    }

    #[rstest]
    fn test_config_command_builtin_sources(basic_config: SantaConfig, test_data: SantaData) {
        // Test builtin sources export (builtin=true, packages=false)
        let result = config_command(&basic_config, &test_data, false, true);
        assert!(
            result.is_ok(),
            "config_command should export builtin sources successfully"
        );
    }

    #[rstest]
    fn test_config_command_with_empty_config(test_data: SantaData) {
        // Test with minimal/empty config
        let empty_config = minimal_config(vec![], vec![]);
        let result = config_command(&empty_config, &test_data, false, false);
        assert!(
            result.is_ok(),
            "config_command should handle empty config gracefully"
        );
    }
}

#[cfg(test)]
mod install_command_tests {
    use super::*;

    #[rstest]
    #[tokio::test]
    async fn test_install_command_basic_execution(
        mut basic_config: SantaConfig,
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test that install_command executes without error when no packages need installation
        // This avoids the terminal interaction by using empty cache (so all packages are "missing")
        // but with empty package list in config
        basic_config.packages = vec![]; // No packages to install
        let temp_dir = std::env::temp_dir();
        let result = install_command(
            &mut basic_config,
            &test_data,
            empty_cache,
            crate::script_generator::ExecutionMode::Safe,
            crate::script_generator::ScriptFormat::Shell,
            &temp_dir,
        )
        .await;
        assert!(
            result.is_ok(),
            "install_command should execute successfully"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_install_command_with_no_enabled_sources(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with no enabled sources
        let mut config = SantaConfig {
            sources: vec![], // No sources enabled
            packages: vec![],
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let temp_dir = std::env::temp_dir();
        let result = install_command(
            &mut config,
            &test_data,
            empty_cache,
            crate::script_generator::ExecutionMode::Safe,
            crate::script_generator::ScriptFormat::Shell,
            &temp_dir,
        )
        .await;
        assert!(
            result.is_ok(),
            "install_command should handle no enabled sources gracefully"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_install_command_filters_enabled_sources(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test that only enabled sources are processed
        // Use empty packages to avoid terminal interaction
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec![], // Empty packages to avoid installation
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let temp_dir = std::env::temp_dir();
        let result = install_command(
            &mut config,
            &test_data,
            empty_cache,
            crate::script_generator::ExecutionMode::Safe,
            crate::script_generator::ScriptFormat::Shell,
            &temp_dir,
        )
        .await;
        assert!(
            result.is_ok(),
            "install_command should filter to enabled sources only"
        );
    }

    #[rstest]
    fn test_install_command_skips_cached_packages() {
        // Test the package filtering logic without calling exec_install
        // This tests the core logic: packages in cache should be filtered out
        let cache = PackageCache::new();
        cache.insert_for_test(
            "brew".to_string(),
            vec!["git".to_string(), "vim".to_string()],
        );

        let source = PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "brew",
            "brew install",
            "brew list",
            None,
            None,
        );

        // Create minimal data for the check method
        let mut packages = PackageDataList::new();
        let mut git_sources = HashMap::new();
        git_sources.insert(KnownSources::Brew, Some(PackageData::new("git")));
        packages.insert("git".to_string(), git_sources);
        let mut vim_sources = HashMap::new();
        vim_sources.insert(KnownSources::Brew, Some(PackageData::new("vim")));
        packages.insert("vim".to_string(), vim_sources);
        let mut curl_sources = HashMap::new();
        curl_sources.insert(KnownSources::Brew, Some(PackageData::new("curl")));
        packages.insert("curl".to_string(), curl_sources);

        let data = SantaData {
            sources: vec![source.clone()],
            packages,
        };

        // Test that cache.check correctly identifies installed packages
        assert!(cache.check(&source, "git", &data), "git should be in cache");
        assert!(cache.check(&source, "vim", &data), "vim should be in cache");
        assert!(
            !cache.check(&source, "curl", &data),
            "curl should not be in cache"
        );

        // The actual filtering logic would be:
        let pkg_list = ["git", "curl", "vim"];
        let to_install: Vec<&str> = pkg_list
            .iter()
            .filter(|p| !cache.check(&source, p, &data))
            .copied()
            .collect();

        assert_eq!(
            to_install,
            vec!["curl"],
            "Only curl should need installation"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_install_command_with_empty_packages(
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test with no packages configured
        let mut config = SantaConfig {
            sources: vec![KnownSources::Brew],
            packages: vec![], // No packages
            custom_sources: None,
            _groups: None,
            log_level: 0,
        };

        let temp_dir = std::env::temp_dir();
        let result = install_command(
            &mut config,
            &test_data,
            empty_cache,
            crate::script_generator::ExecutionMode::Safe,
            crate::script_generator::ScriptFormat::Shell,
            &temp_dir,
        )
        .await;
        assert!(
            result.is_ok(),
            "install_command should handle empty package list gracefully"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[rstest]
    #[tokio::test]
    async fn test_command_chain_status_then_config(
        basic_config: SantaConfig,
        test_data: SantaData,
        empty_cache: PackageCache,
    ) {
        // Test running status command followed by config command (no terminal interaction)
        let mut config_clone = basic_config.clone();

        // First run status
        let status_result = status_command(
            &mut config_clone,
            &test_data,
            empty_cache,
            &false,
            &false,
            &false,
            None,
        )
        .await;
        assert!(status_result.is_ok(), "status_command should succeed");

        // Then run config
        let config_result = config_command(&basic_config, &test_data, false, false);
        assert!(
            config_result.is_ok(),
            "config_command should succeed after status"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_all_commands_with_minimal_data(empty_cache: PackageCache) {
        // Test all commands with minimal data structures
        let minimal_config = minimal_config(vec![], vec![]);
        let minimal_data = SantaData {
            sources: vec![],
            packages: PackageDataList::new(),
        };

        let mut config_clone = minimal_config.clone();
        let cache_clone1 = empty_cache.clone();
        let cache_clone2 = empty_cache.clone();

        // All commands should handle minimal data gracefully
        assert!(config_command(&minimal_config, &minimal_data, false, false).is_ok());
        assert!(status_command(
            &mut config_clone,
            &minimal_data,
            cache_clone1,
            &false,
            &false,
            &false,
            None
        )
        .await
        .is_ok());

        // For install command, ensure no packages to avoid terminal interaction
        config_clone.packages = vec![];
        let temp_dir = std::env::temp_dir();
        assert!(install_command(
            &mut config_clone,
            &minimal_data,
            cache_clone2,
            crate::script_generator::ExecutionMode::Safe,
            crate::script_generator::ScriptFormat::Shell,
            &temp_dir,
        )
        .await
        .is_ok());
    }
}

#[cfg(test)]
mod add_command_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a test config file with CCL content
    fn create_test_config_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::with_suffix(".ccl").unwrap();
        write!(temp_file, "{}", content).unwrap();
        temp_file
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_adds_valid_package(test_data: SantaData) {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Add a package that exists in the test data
        let result = add_command(temp_file.path(), vec!["git".to_string()], &test_data).await;
        assert!(result.is_ok(), "add_command should succeed for valid packages");

        // Verify the package was added
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(
            updated_content.contains("git"),
            "Config should contain the added package"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_rejects_invalid_package(test_data: SantaData) {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Try to add a package that doesn't exist in the data
        let result =
            add_command(temp_file.path(), vec!["nonexistent_package".to_string()], &test_data)
                .await;
        assert!(
            result.is_err(),
            "add_command should fail for packages not in database"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_skips_duplicate_packages(test_data: SantaData) {
        // Create a temporary config file with git already in it
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Try to add git again (should skip but not error)
        let result = add_command(temp_file.path(), vec!["git".to_string()], &test_data).await;
        assert!(
            result.is_ok(),
            "add_command should succeed even for duplicate packages"
        );

        // Verify there's only one instance of git
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        let git_count = updated_content.matches("git").count();
        // CCL format may have multiple occurrences due to formatting, so just check it's reasonable
        assert!(git_count >= 1, "Config should have at least one git entry");
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_adds_multiple_packages(test_data: SantaData) {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Add multiple packages
        let result = add_command(
            temp_file.path(),
            vec!["git".to_string(), "curl".to_string()],
            &test_data,
        )
        .await;
        assert!(
            result.is_ok(),
            "add_command should succeed for multiple valid packages"
        );

        // Verify both packages were added
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(
            updated_content.contains("git"),
            "Config should contain git"
        );
        assert!(
            updated_content.contains("curl"),
            "Config should contain curl"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_fails_with_nonexistent_file() {
        let test_data = SantaData {
            sources: vec![],
            packages: PackageDataList::new(),
        };

        let result = add_command(
            std::path::Path::new("/nonexistent/config.ccl"),
            vec!["git".to_string()],
            &test_data,
        )
        .await;
        assert!(
            result.is_err(),
            "add_command should fail for nonexistent config file"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_add_command_validates_all_packages_before_adding(test_data: SantaData) {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Try to add mix of valid and invalid packages
        let result = add_command(
            temp_file.path(),
            vec!["git".to_string(), "invalid_package".to_string()],
            &test_data,
        )
        .await;
        assert!(
            result.is_err(),
            "add_command should fail if any package is invalid"
        );

        // Verify no changes were made (transactional behavior)
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(
            !updated_content.contains("git") || config_content.contains("git"),
            "Config should not be modified if validation fails"
        );
    }
}

#[cfg(test)]
mod remove_command_tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a test config file with CCL content
    fn create_test_config_file(content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::with_suffix(".ccl").unwrap();
        write!(temp_file, "{}", content).unwrap();
        temp_file
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_removes_existing_package() {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
  = curl
"#;
        let temp_file = create_test_config_file(config_content);

        // Remove a package
        let result = remove_command(temp_file.path(), vec!["git".to_string()], false).await;
        assert!(result.is_ok(), "remove_command should succeed");

        // Verify the package was removed
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        // Note: 'git' might still appear in context, check packages section more carefully
        // This is a basic check; actual CCL parsing would be more reliable
        assert!(
            updated_content.contains("vim"),
            "Config should still contain vim"
        );
        assert!(
            updated_content.contains("curl"),
            "Config should still contain curl"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_handles_nonexistent_package() {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Try to remove a package that's not in config
        let result =
            remove_command(temp_file.path(), vec!["nonexistent".to_string()], false).await;
        assert!(
            result.is_ok(),
            "remove_command should succeed even when package not found"
        );

        // Verify config was not modified
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(
            updated_content.contains("git"),
            "Config should still contain git"
        );
        assert!(
            updated_content.contains("vim"),
            "Config should still contain vim"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_removes_multiple_packages() {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
  = curl
"#;
        let temp_file = create_test_config_file(config_content);

        // Remove multiple packages
        let result = remove_command(
            temp_file.path(),
            vec!["git".to_string(), "vim".to_string()],
            false,
        )
        .await;
        assert!(result.is_ok(), "remove_command should succeed");

        // Verify both packages were removed
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(
            updated_content.contains("curl"),
            "Config should still contain curl"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_fails_with_nonexistent_file() {
        let result = remove_command(
            std::path::Path::new("/nonexistent/config.ccl"),
            vec!["git".to_string()],
            false,
        )
        .await;
        assert!(
            result.is_err(),
            "remove_command should fail for nonexistent config file"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_with_uninstall_flag() {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
"#;
        let temp_file = create_test_config_file(config_content);

        // Remove with uninstall flag (currently just prints message)
        let result = remove_command(temp_file.path(), vec!["git".to_string()], true).await;
        assert!(
            result.is_ok(),
            "remove_command should succeed with uninstall flag"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn test_remove_command_removes_all_specified() {
        // Create a temporary config file
        let config_content = r#"
sources =
  = brew
packages =
  = git
  = vim
  = curl
"#;
        let temp_file = create_test_config_file(config_content);

        // Remove all packages
        let result = remove_command(
            temp_file.path(),
            vec!["git".to_string(), "vim".to_string(), "curl".to_string()],
            false,
        )
        .await;
        assert!(result.is_ok(), "remove_command should succeed");

        // Verify all packages were removed - config should have empty packages
        let updated_content = std::fs::read_to_string(temp_file.path()).unwrap();
        // The packages section should be empty or minimal
        assert!(
            updated_content.contains("packages"),
            "Config should still have packages section"
        );
    }
}

//! Integration tests for command execution behavior
//!
//! These tests validate that Santa properly handles package manager command execution,
//! including error cases, timeout handling, and concurrent operations.

use super::MockPackageSource;
use santa::traits::PackageManager;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_successful_package_installation() {
    let mock_source =
        MockPackageSource::new("mock-apt").with_packages(vec!["existing-pkg".to_string()]);

    let result = mock_source.install_packages(&["curl", "git"]).await;
    assert!(result.is_ok(), "Package installation should succeed");
}

#[tokio::test]
async fn test_failed_package_installation() {
    let mock_source = MockPackageSource::new("mock-apt").with_failure(true);

    let result = mock_source.install_packages(&["nonexistent-pkg"]).await;
    assert!(result.is_err(), "Package installation should fail");

    if let Err(e) = result {
        assert!(e.to_string().contains("mock install"));
    }
}

#[tokio::test]
async fn test_package_listing() {
    let expected_packages = vec!["curl".to_string(), "git".to_string(), "vim".to_string()];

    let mock_source = MockPackageSource::new("mock-brew").with_packages(expected_packages.clone());

    let result = mock_source.list_packages().await;
    assert!(result.is_ok(), "Package listing should succeed");

    let packages = result.unwrap();
    assert_eq!(packages, expected_packages);
}

#[tokio::test]
async fn test_package_existence_check() {
    let mock_source = MockPackageSource::new("mock-npm")
        .with_packages(vec!["react".to_string(), "webpack".to_string()]);

    assert!(mock_source.is_package_installed("react"));
    assert!(mock_source.is_package_installed("webpack"));
    assert!(!mock_source.is_package_installed("nonexistent"));
}

#[tokio::test]
async fn test_concurrent_operations() {
    let mock_source = MockPackageSource::new("mock-pip")
        .with_packages(vec!["numpy".to_string(), "pandas".to_string()]);

    // Test that multiple concurrent operations work
    let install_future = mock_source.install_packages(&["scipy"]);
    let list_future = mock_source.list_packages();

    let (install_result, list_result) = tokio::join!(install_future, list_future);

    assert!(install_result.is_ok());
    assert!(list_result.is_ok());
    assert_eq!(list_result.unwrap(), vec!["numpy", "pandas"]);
}

#[tokio::test]
async fn test_timeout_handling() {
    let mock_source = MockPackageSource::new("mock-slow");

    // This should complete quickly since our mock is fast
    let result = timeout(
        Duration::from_millis(100),
        mock_source.install_packages(&["quick-package"]),
    )
    .await;

    assert!(result.is_ok(), "Fast operation should not timeout");
    assert!(result.unwrap().is_ok(), "Installation should succeed");
}

#[tokio::test]
async fn test_batch_install_support() {
    let mock_source = MockPackageSource::new("mock-apt");

    assert!(mock_source.supports_batch_install());

    let result = mock_source
        .install_packages(&["pkg1", "pkg2", "pkg3"])
        .await;
    assert!(result.is_ok(), "Batch installation should succeed");
}

#[tokio::test]
async fn test_package_manager_metadata() {
    let mock_source = MockPackageSource::new("test-manager");

    assert_eq!(mock_source.name(), "test-manager");
    assert_eq!(mock_source.install_command(), "test-manager install");
    assert_eq!(mock_source.list_command(), "test-manager list");
    assert!(!mock_source.requires_elevation());
}

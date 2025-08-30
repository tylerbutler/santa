//! Integration test modules for Santa Package Manager
//! 
//! This module contains integration tests that test the full behavior
//! of Santa components working together, including mock implementations
//! for safe testing without system package manager execution.

pub mod command_execution;
pub mod config_hot_reload;
pub mod cache_behavior;

use santa::errors::{Result, SantaError};
use santa::traits::PackageManager;

/// Mock package source for testing without executing real commands
#[derive(Debug, Clone)]
pub struct MockPackageSource {
    pub name: String,
    pub packages: Vec<String>,
    pub should_fail: bool,
    pub install_cmd: String,
    pub list_cmd: String,
}

impl MockPackageSource {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            packages: Vec::new(),
            should_fail: false,
            install_cmd: format!("{} install", name),
            list_cmd: format!("{} list", name),
        }
    }

    pub fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }

    pub fn with_packages(mut self, packages: Vec<String>) -> Self {
        self.packages = packages;
        self
    }
}

impl PackageManager for MockPackageSource {
    type Error = SantaError;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn install_command(&self) -> &str {
        &self.install_cmd
    }

    fn list_command(&self) -> &str {
        &self.list_cmd
    }

    fn install_packages(&self, packages: &[&str]) -> impl std::future::Future<Output = Result<()>> + Send {
        let should_fail = self.should_fail;
        let package_list = packages.join(" ");
        
        async move {
            if should_fail {
                Err(SantaError::CommandFailed(format!("mock install {}", package_list)))
            } else {
                // Simulate successful installation
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Ok(())
            }
        }
    }

    fn list_packages(&self) -> impl std::future::Future<Output = Result<Vec<String>>> + Send {
        let packages = self.packages.clone();
        let should_fail = self.should_fail;
        
        async move {
            if should_fail {
                Err(SantaError::CommandFailed("mock list".to_string()))
            } else {
                Ok(packages)
            }
        }
    }

    fn is_package_installed(&self, package: &str) -> bool {
        self.packages.iter().any(|p| p == package)
    }

    fn supports_batch_install(&self) -> bool {
        true
    }
}
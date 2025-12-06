//! Safe script generation for package manager operations.
//!
//! This module provides the core script generation functionality that makes Santa
//! secure by default. Instead of directly executing potentially dangerous commands,
//! Santa generates platform-specific scripts that can be reviewed before execution.
//!
//! # Architecture
//!
//! - [`ScriptGenerator`]: MiniJinja-based template engine for script generation
//! - [`ExecutionMode`]: Safe (script generation) vs Execute (direct execution)
//! - [`ScriptFormat`]: Platform-specific script formats (Shell, PowerShell, Batch)
//!
//! # Security
//!
//! All user inputs are sanitized using:
//! - Shell escaping via `shell-escape` crate
//! - PowerShell escaping with custom filters
//! - Package name validation
//! - Template-based command construction
//!
//! # Examples
//!
//! ```rust,no_run
//! use santa::script_generator::{ScriptGenerator, ScriptFormat};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let generator = ScriptGenerator::new()?;
//! let packages = vec!["git".to_string(), "rust".to_string()];
//!
//! // Generate a safe shell script
//! let script = generator.generate_install_script(
//!     &packages,
//!     "brew",
//!     ScriptFormat::Shell,
//!     "homebrew"
//! )?;
//!
//! // Script can now be reviewed and executed manually
//! println!("{}", script);
//! # Ok(())
//! # }
//! ```

use crate::errors::{Result, SantaError};
use chrono::Utc;
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use shell_escape::escape;

/// Execution modes for Santa - determines whether to execute directly or generate scripts.
///
/// The default mode is [`ExecutionMode::Safe`], which generates scripts that can be
/// reviewed before execution. [`ExecutionMode::Execute`] directly runs commands and
/// requires explicit opt-in
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    /// Generate scripts only (safe default mode)
    #[default]
    Safe,
    /// Execute commands directly (dangerous mode, requires opt-in)
    Execute,
}

/// Script formats for different platforms and shells.
///
/// Santa automatically detects the appropriate format based on the current
/// platform, but users can explicitly specify a format if needed.
///
/// # Examples
///
/// ```rust
/// use santa::script_generator::ScriptFormat;
///
/// // Auto-detect based on platform
/// let format = ScriptFormat::auto_detect();
///
/// // Get file extension
/// assert_eq!(format.extension(), if cfg!(windows) { "ps1" } else { "sh" });
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptFormat {
    /// Unix shell script (.sh) for Linux and macOS
    Shell,
    /// Windows PowerShell script (.ps1) - modern Windows default
    PowerShell,
    /// Windows Batch file (.bat) - legacy Windows fallback
    Batch,
}

impl ScriptFormat {
    /// Choose appropriate format based on current platform
    pub fn auto_detect() -> Self {
        if cfg!(windows) {
            ScriptFormat::PowerShell
        } else {
            ScriptFormat::Shell
        }
    }

    /// Get file extension for this script format
    pub fn extension(&self) -> &'static str {
        match self {
            ScriptFormat::Shell => "sh",
            ScriptFormat::PowerShell => "ps1",
            ScriptFormat::Batch => "bat",
        }
    }

    /// Get template name for this script format
    pub fn install_template_name(&self) -> &'static str {
        match self {
            ScriptFormat::Shell => "install.sh",
            ScriptFormat::PowerShell => "install.ps1",
            ScriptFormat::Batch => "install.bat",
        }
    }

    /// Get check template name for this script format
    pub fn check_template_name(&self) -> &'static str {
        match self {
            ScriptFormat::Shell => "check.sh",
            ScriptFormat::PowerShell => "check.ps1",
            ScriptFormat::Batch => "check.bat",
        }
    }
}

/// Script generator using MiniJinja templates for safe script generation.
///
/// The generator uses embedded MiniJinja templates to create platform-specific
/// scripts with proper escaping and validation. This design prevents command
/// injection attacks and allows users to review generated scripts before execution.
///
/// # Security Features
///
/// - Shell escaping for Unix commands
/// - PowerShell escaping for Windows commands
/// - Package name validation
/// - Template-based construction (no string interpolation)
///
/// # Examples
///
/// ```rust,no_run
/// use santa::script_generator::{ScriptGenerator, ScriptFormat};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let generator = ScriptGenerator::new()?;
///
/// // Generate installation script
/// let packages = vec!["git".to_string(), "curl".to_string()];
/// let script = generator.generate_install_script(
///     &packages,
///     "apt-get",
///     ScriptFormat::Shell,
///     "apt"
/// )?;
///
/// // Write script to file or execute
/// std::fs::write("install.sh", script)?;
/// # Ok(())
/// # }
/// ```
pub struct ScriptGenerator {
    env: Environment<'static>,
}

impl ScriptGenerator {
    /// Create new script generator with built-in templates.
    ///
    /// Initializes the MiniJinja template engine with embedded templates for
    /// Shell, PowerShell, and Batch formats, and registers custom filters
    /// for secure escaping.
    ///
    /// # Returns
    ///
    /// Returns a new [`ScriptGenerator`] or a [`SantaError::Template`] if
    /// template initialization fails.
    pub fn new() -> Result<Self> {
        let mut env = Environment::new();

        // Add built-in templates for different script formats
        env.add_template("install.sh", include_str!("../templates/install.sh.tera"))
            .map_err(|e| SantaError::Template(e.to_string()))?;

        env.add_template("install.ps1", include_str!("../templates/install.ps1.tera"))
            .map_err(|e| SantaError::Template(e.to_string()))?;

        env.add_template("check.sh", include_str!("../templates/check.sh.tera"))
            .map_err(|e| SantaError::Template(e.to_string()))?;

        env.add_template("check.ps1", include_str!("../templates/check.ps1.tera"))
            .map_err(|e| SantaError::Template(e.to_string()))?;

        // Register custom filters for safe escaping
        env.add_filter("shell_escape", shell_escape_filter);
        env.add_filter("powershell_escape", powershell_escape_filter);
        env.add_filter("validate_package", validate_package_filter);

        Ok(Self { env })
    }

    /// Generate installation script for given packages and manager
    pub fn generate_install_script(
        &self,
        packages: &[String],
        manager: &str,
        format: ScriptFormat,
        source_name: &str,
    ) -> Result<String> {
        let template_name = format.install_template_name();
        let template = self
            .env
            .get_template(template_name)
            .map_err(|e| SantaError::Template(e.to_string()))?;

        let context = minijinja::context! {
            packages => packages,
            manager => manager,
            source_name => source_name,
            timestamp => Utc::now().to_rfc3339(),
            version => env!("CARGO_PKG_VERSION"),
            package_count => packages.len(),
        };

        template.render(context).map_err(|e| {
            SantaError::Template(format!(
                "Failed to render {} template: {}",
                template_name, e
            ))
        })
    }

    /// Generate check script for listing installed packages
    pub fn generate_check_script(
        &self,
        manager: &str,
        check_command: &str,
        format: ScriptFormat,
        source_name: &str,
    ) -> Result<String> {
        let template_name = format.check_template_name();
        let template = self
            .env
            .get_template(template_name)
            .map_err(|e| SantaError::Template(e.to_string()))?;

        let context = minijinja::context! {
            manager => manager,
            check_command => check_command,
            source_name => source_name,
            timestamp => Utc::now().to_rfc3339(),
            version => env!("CARGO_PKG_VERSION"),
        };

        template.render(context).map_err(|e| {
            SantaError::Template(format!(
                "Failed to render {} template: {}",
                template_name, e
            ))
        })
    }

    /// Generate script filename with timestamp
    pub fn generate_filename(prefix: &str, format: &ScriptFormat) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}.{}", prefix, timestamp, format.extension())
    }
}

impl Default for ScriptGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to initialize script generator with built-in templates")
    }
}

/// MiniJinja filter for shell escaping using shell-escape crate
fn shell_escape_filter(value: String) -> String {
    escape(value.into()).into_owned()
}

/// MiniJinja filter for PowerShell argument escaping
fn powershell_escape_filter(value: String) -> String {
    escape_powershell_arg(&value)
}

/// MiniJinja filter for package name validation
fn validate_package_filter(value: String) -> std::result::Result<String, minijinja::Error> {
    if is_safe_package_name(&value) {
        Ok(value)
    } else {
        Err(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("Package name contains dangerous characters: {}", value),
        ))
    }
}

/// Escape PowerShell arguments safely
fn escape_powershell_arg(arg: &str) -> String {
    // PowerShell single quotes prevent most variable expansion
    // Escape single quotes by doubling them
    format!("'{}'", arg.replace("'", "''"))
}

/// Check if a package name is safe (basic validation)
fn is_safe_package_name(name: &str) -> bool {
    // Reject obviously dangerous patterns
    let dangerous_patterns = &[
        "$(", "`", ">&", "|", ";", "&&", "||", "../", "..\\", "/dev/", "C:\\", "\\\\", "curl",
        "wget", "rm -rf", "del /s",
    ];

    for pattern in dangerous_patterns {
        if name.contains(pattern) {
            return false;
        }
    }

    // Additional checks: no null bytes, control characters
    !name.chars().any(|c| c.is_control() || c == '\0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_format_auto_detect() {
        let format = ScriptFormat::auto_detect();
        if cfg!(windows) {
            assert_eq!(format, ScriptFormat::PowerShell);
        } else {
            assert_eq!(format, ScriptFormat::Shell);
        }
    }

    #[test]
    fn test_script_format_extensions() {
        assert_eq!(ScriptFormat::Shell.extension(), "sh");
        assert_eq!(ScriptFormat::PowerShell.extension(), "ps1");
        assert_eq!(ScriptFormat::Batch.extension(), "bat");
    }

    #[test]
    fn test_execution_mode_default() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::Safe);
    }

    #[test]
    fn test_powershell_escaping() {
        assert_eq!(escape_powershell_arg("simple"), "'simple'");
        assert_eq!(escape_powershell_arg("with'quote"), "'with''quote'");
        assert_eq!(
            escape_powershell_arg("complex'test'case"),
            "'complex''test''case'"
        );
    }

    #[test]
    fn test_package_name_validation() {
        // Safe package names
        assert!(is_safe_package_name("git"));
        assert!(is_safe_package_name("node-sass"));
        assert!(is_safe_package_name("package_with_underscores"));
        assert!(is_safe_package_name("package-with-dashes"));

        // Dangerous package names
        assert!(!is_safe_package_name("package; rm -rf /"));
        assert!(!is_safe_package_name("$(evil_command)"));
        assert!(!is_safe_package_name("package`with`backticks"));
        assert!(!is_safe_package_name("../../../etc/passwd"));
        assert!(!is_safe_package_name("curl evil.com"));
    }

    #[test]
    fn test_script_generator_creation() {
        let generator = ScriptGenerator::new();
        assert!(
            generator.is_ok(),
            "Script generator should initialize successfully"
        );
    }

    #[test]
    fn test_filename_generation() {
        let filename = ScriptGenerator::generate_filename("santa_install", &ScriptFormat::Shell);
        assert!(filename.starts_with("santa_install_"));
        assert!(filename.ends_with(".sh"));

        let ps_filename =
            ScriptGenerator::generate_filename("santa_check", &ScriptFormat::PowerShell);
        assert!(ps_filename.starts_with("santa_check_"));
        assert!(ps_filename.ends_with(".ps1"));
    }

    #[test]
    fn test_script_format_template_names() {
        assert_eq!(ScriptFormat::Shell.install_template_name(), "install.sh");
        assert_eq!(
            ScriptFormat::PowerShell.install_template_name(),
            "install.ps1"
        );
        assert_eq!(ScriptFormat::Batch.install_template_name(), "install.bat");

        assert_eq!(ScriptFormat::Shell.check_template_name(), "check.sh");
        assert_eq!(ScriptFormat::PowerShell.check_template_name(), "check.ps1");
        assert_eq!(ScriptFormat::Batch.check_template_name(), "check.bat");
    }

    #[test]
    fn test_generate_install_script_shell() {
        let generator = ScriptGenerator::new().unwrap();
        let packages = vec!["git".to_string(), "curl".to_string()];
        let script = generator
            .generate_install_script(&packages, "brew", ScriptFormat::Shell, "homebrew")
            .unwrap();

        assert!(script.contains("brew"), "Script should contain manager name");
        assert!(script.contains("git"), "Script should contain package name");
        assert!(script.contains("curl"), "Script should contain package name");
    }

    #[test]
    fn test_generate_install_script_powershell() {
        let generator = ScriptGenerator::new().unwrap();
        let packages = vec!["git".to_string()];
        let script = generator
            .generate_install_script(&packages, "choco", ScriptFormat::PowerShell, "chocolatey")
            .unwrap();

        assert!(
            script.contains("choco"),
            "Script should contain manager name"
        );
        assert!(script.contains("git"), "Script should contain package name");
    }

    #[test]
    fn test_generate_check_script_shell() {
        let generator = ScriptGenerator::new().unwrap();
        let script = generator
            .generate_check_script("brew", "brew list", ScriptFormat::Shell, "homebrew")
            .unwrap();

        assert!(
            script.contains("brew list"),
            "Script should contain check command"
        );
    }

    #[test]
    fn test_generate_check_script_powershell() {
        let generator = ScriptGenerator::new().unwrap();
        let script = generator
            .generate_check_script("choco", "choco list", ScriptFormat::PowerShell, "chocolatey")
            .unwrap();

        assert!(
            script.contains("choco list"),
            "Script should contain check command"
        );
    }

    #[test]
    fn test_shell_escape_filter() {
        // Test that shell escaping works correctly
        let result = shell_escape_filter("simple".to_string());
        assert!(!result.is_empty());

        let result_space = shell_escape_filter("with space".to_string());
        assert!(result_space.contains("with space"));
    }

    #[test]
    fn test_powershell_escape_filter() {
        let result = powershell_escape_filter("simple".to_string());
        assert_eq!(result, "'simple'");

        let result_quote = powershell_escape_filter("with'quote".to_string());
        assert_eq!(result_quote, "'with''quote'");
    }

    #[test]
    fn test_validate_package_filter_valid() {
        let result = validate_package_filter("git".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git");
    }

    #[test]
    fn test_validate_package_filter_invalid() {
        let result = validate_package_filter("$(evil)".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_script_generator_default() {
        let generator = ScriptGenerator::default();
        let packages = vec!["test".to_string()];
        // Should not panic and should produce valid output
        let result =
            generator.generate_install_script(&packages, "brew", ScriptFormat::Shell, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_install_script_empty_packages() {
        let generator = ScriptGenerator::new().unwrap();
        let packages: Vec<String> = vec![];
        let script = generator
            .generate_install_script(&packages, "brew", ScriptFormat::Shell, "homebrew")
            .unwrap();

        // Should still generate a valid script structure
        assert!(!script.is_empty());
    }

    #[test]
    fn test_generate_install_script_includes_metadata() {
        let generator = ScriptGenerator::new().unwrap();
        let packages = vec!["git".to_string()];
        let script = generator
            .generate_install_script(&packages, "brew", ScriptFormat::Shell, "homebrew")
            .unwrap();

        // Script should include source name and version info
        assert!(
            script.contains("homebrew"),
            "Script should contain source name"
        );
    }
}

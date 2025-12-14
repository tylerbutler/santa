# Uninstall Functionality Implementation Plan

**Status**: Planned
**Priority**: Feature Enhancement
**Estimated Scope**: ~300 lines across 8 files

## Overview

Implement package uninstall functionality that mirrors the existing install architecture. Santa uses a safe-by-default script generation model, so uninstall should follow the same pattern.

## Architecture

Santa's execution model:
- **Safe mode (default)**: Generate platform-specific scripts for user review
- **Execute mode**: Direct execution with confirmation (explicit opt-in)

## Implementation Steps

### 1. Add `uninstall_command` Field to Data Structures

**File**: `crates/santa-cli/src/sources.rs`

Add to `SourceOverride` (around line 305):
```rust
pub uninstall_command: Option<String>,
```

Add to `PackageSource` (around line 320):
```rust
#[serde(alias = "uninstall")]
uninstall_command: String,
```

Update `From<ConfigPackageSource>` impl to include the new field.

### 2. Add Accessor Methods

**File**: `crates/santa-cli/src/sources.rs`

Add after other command accessors (~line 557):
```rust
/// Returns the configured uninstall command, respecting platform overrides.
#[must_use]
pub fn uninstall_command(&self) -> String {
    match self.get_override_for_current_platform() {
        Some(ov) => ov.uninstall_command.clone().unwrap_or_else(|| self.uninstall_command.clone()),
        None => self.uninstall_command.clone(),
    }
}

#[must_use]
pub fn uninstall_packages_command(&self, packages: Vec<String>) -> String {
    let escaped_packages: Vec<String> = packages
        .iter()
        .map(|pkg| self.sanitize_package_name(pkg))
        .collect();
    format!("{} {}", self.uninstall_command(), escaped_packages.join(" "))
}
```

### 3. Create Uninstall Script Templates

**New files**:
- `crates/santa-cli/templates/uninstall.sh.tera`
- `crates/santa-cli/templates/uninstall.ps1.tera`

Templates should:
- Check if package manager is available
- Loop through packages and uninstall each
- Handle errors gracefully
- Include generated timestamp and version info

Example structure for shell:
```bash
#!/bin/bash
set -euo pipefail

echo "🎅 Santa Package Uninstall - {{ source_name }}"
{% for package in packages %}
{{ manager }} uninstall {{ package | shell_escape }}
{% endfor %}
```

### 4. Update ScriptGenerator

**File**: `crates/santa-cli/src/script_generator.rs`

Add template name method:
```rust
pub fn uninstall_template_name(&self) -> &'static str {
    match self {
        ScriptFormat::Shell => "uninstall.sh",
        ScriptFormat::PowerShell => "uninstall.ps1",
        ScriptFormat::Batch => "uninstall.bat",
    }
}
```

Register templates in `ScriptGenerator::new()`:
```rust
env.add_template("uninstall.sh", include_str!("../templates/uninstall.sh.tera"))?;
env.add_template("uninstall.ps1", include_str!("../templates/uninstall.ps1.tera"))?;
```

Add generation method:
```rust
pub fn generate_uninstall_script(
    &self,
    packages: &[String],
    manager: &str,
    format: ScriptFormat,
    source_name: &str,
) -> Result<String>
```

### 5. Add `exec_uninstall()` to PackageSource

**File**: `crates/santa-cli/src/sources.rs`

Similar to `exec_install()` but for uninstall operations. Should:
- Check for empty package list early
- Rename packages using `data.name_for()`
- Handle Safe mode (script generation)
- Handle Execute mode (direct execution with confirmation)
- Use 5-minute timeout for uninstall operations

### 6. Create `uninstall_command()` Function

**File**: `crates/santa-cli/src/commands.rs`

Similar to install_command pattern:
```rust
pub async fn uninstall_command(
    config: &mut SantaConfig,
    data: &SantaData,
    package_names: Vec<String>,
    cache: PackageCache,
    execution_mode: ExecutionMode,
    script_format: ScriptFormat,
    output_dir: &std::path::Path,
) -> Result<()>
```

Should:
- Filter sources to enabled ones
- Cache data for sources concurrently
- For each source, find installed packages that match request
- Generate uninstall scripts or execute commands

### 7. Wire Up in `remove_command`

**File**: `crates/santa-cli/src/commands.rs`

Replace the TODO at line 546:
```rust
if uninstall {
    let data = SantaData::default();
    let cache = PackageCache::new();

    uninstall_command(
        &mut config,
        &data,
        package_names.clone(),
        cache,
        ExecutionMode::Safe,
        ScriptFormat::auto_detect(),
        std::path::Path::new("."),
    )
    .await
    .context("Failed to uninstall packages")?;
}
```

### 8. Update Data Sources with Uninstall Commands

**File**: `crates/santa-cli/data/sources/*.ccl`

Add uninstall commands for each source:
```ccl
brew =
  emoji = 🍺
  install = brew install {package}
  uninstall = brew uninstall {package}
  check = brew leaves --installed-on-request

pacman =
  emoji = 👾
  install = sudo pacman -Syyu {package}
  uninstall = sudo pacman -R {package}
  check = pacman -Qe | cut -f 1 -d " "
```

### 9. Update ConfigPackageSource

Add `uninstall_command: String` to the configuration struct.

## Testing Strategy

1. Unit tests for new accessor methods
2. Integration tests for script generation
3. E2E tests for `santa remove --uninstall` command
4. Security tests for command injection prevention

## Verification Commands

```bash
# After implementation
cargo check
cargo test
cargo clippy -- -A clippy::needless_return -D warnings

# Manual testing
santa remove --uninstall git vim  # Safe mode - generates script
santa remove --uninstall --execute git vim  # Direct execution
```

## Security Considerations

- All package names must be sanitized with `shell-escape`
- Script generation prevents injection by design
- Execute mode requires explicit confirmation
- 5-minute timeout prevents hanging operations

## Notes

- Mirrors existing install architecture for consistency
- Safe-by-default approach maintained
- Platform-specific overrides supported
- Async execution with proper timeout handling

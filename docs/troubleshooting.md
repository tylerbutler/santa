# Santa Troubleshooting Guide

Common issues and solutions when using Santa.

## Configuration Issues

### Configuration Not Loading

**Symptom**: Changes to `~/.config/santa/config.ccl` don't appear to take effect.

**Solutions**:

1. Verify configuration file location:
   ```bash
   # Check default location exists
   ls -la ~/.config/santa/config.ccl

   # Verify config is loaded
   santa config
   ```

2. Check for syntax errors:
   ```bash
   # Santa will report CCL parsing errors
   santa config
   ```

3. Verify environment variable isn't overriding:
   ```bash
   # Check if SANTA_CONFIG is set
   echo $SANTA_CONFIG

   # If set, either unset it or use that file
   unset SANTA_CONFIG
   ```

4. Test with built-in configuration only:
   ```bash
   santa --builtin-only config
   ```

### CCL Syntax Errors

**Symptom**: `Error: Failed to parse configuration`

**Common causes**:

1. **Incorrect indentation**: CCL uses 2-space indentation
   ```ccl
   # Wrong
   sources =
   = brew

   # Correct
   sources =
     = brew
   ```

2. **Missing `=` signs**:
   ```ccl
   # Wrong
   sources
     brew

   # Correct
   sources =
     = brew
   ```

3. **Invalid comment syntax**:
   ```ccl
   # Wrong - uses # instead of /=
   # This is a comment

   # Correct
   /= This is a comment
   ```

**Debug steps**:

1. Simplify configuration to minimal example
2. Add complexity incrementally
3. Check indentation carefully (use spaces, not tabs)
4. Refer to [Configuration Guide](configuration.md) for syntax examples

### Package Not Found

**Symptom**: Package shows as missing even though it's installed.

**Causes and solutions**:

1. **Package installed from different source**:
   ```bash
   # Check all sources for the package
   santa status --all

   # Package might be in a different source than expected
   ```

2. **Package has different name in source**:
   ```ccl
   /= Some packages have different names
   /= Example: fd vs fd-find

   fd-find =
     brew = fd
     cargo = fd-find
     apt = fd-find
   ```

3. **Source not in configuration**:
   ```bash
   # Check configured sources
   santa config

   # Add missing source to config
   ```

4. **Package manager not detected**:
   ```bash
   # Verify package manager is available
   which brew
   which cargo
   which apt

   # Santa filters to available sources automatically
   ```

## Installation Issues

### Script Generation Fails

**Symptom**: `santa install` completes but no script is generated.

**Solutions**:

1. Check output directory exists and is writable:
   ```bash
   # Default location
   ls -la ~/.santa/scripts/

   # Create if missing
   mkdir -p ~/.santa/scripts

   # Check permissions
   chmod 755 ~/.santa/scripts
   ```

2. Specify custom output directory:
   ```bash
   santa install --output-dir ./scripts
   ```

3. Check for errors with verbose logging:
   ```bash
   santa install -vv
   ```

### Script Won't Execute

**Symptom**: Generated script has errors when run.

**Solutions**:

1. Review script contents first:
   ```bash
   cat ~/.santa/scripts/install_*.sh
   ```

2. Check script permissions:
   ```bash
   chmod +x ~/.santa/scripts/install_*.sh
   ```

3. Run with explicit shell:
   ```bash
   # Shell script
   bash ~/.santa/scripts/install_*.sh

   # PowerShell
   powershell -ExecutionPolicy Bypass -File install_*.ps1
   ```

4. Package manager not in PATH:
   ```bash
   # Verify package manager is accessible
   which brew
   which cargo

   # Add to PATH if needed
   export PATH="/usr/local/bin:$PATH"
   ```

### Execute Mode Fails

**Symptom**: `santa install -x` fails with command errors.

**Solutions**:

1. **First, try safe mode** to see what commands would run:
   ```bash
   santa install
   cat ~/.santa/scripts/install_*.sh
   ```

2. Check package manager is working:
   ```bash
   # Test package manager directly
   brew --version
   cargo --version
   apt --version
   ```

3. Check for permission issues:
   ```bash
   # Some package managers need sudo
   sudo santa install -x
   ```

4. Use safe mode and review commands before executing.

## Source Management Issues

### Source Update Fails

**Symptom**: `santa sources update` fails to download definitions.

**Solutions**:

1. Check network connectivity:
   ```bash
   # Verify internet access
   curl -I https://github.com
   ```

2. Check GitHub API access:
   ```bash
   # Test GitHub API
   curl https://api.github.com/repos/tylerbutler/santa/releases
   ```

3. Verify download directory is writable:
   ```bash
   # Default location
   ls -la ~/.local/share/santa/sources/

   # Create if needed
   mkdir -p ~/.local/share/santa/sources
   ```

4. Check proxy/firewall settings if behind corporate network.

### Unknown Source Warning

**Symptom**: `Warning: Unknown source 'xyz' in configuration`

**Cause**: Configuration references a source Santa doesn't recognize.

**Solutions**:

1. Check source name spelling:
   ```ccl
   /= Common typos
   sources =
     = homebrew    /= Wrong - should be 'brew'
     = rust        /= Wrong - should be 'cargo'
   ```

2. Verify supported sources:
   ```bash
   santa sources list
   ```

3. Update source definitions:
   ```bash
   santa sources update
   ```

## Package Status Issues

### Status Shows All Missing

**Symptom**: `santa status` shows all packages as missing, but they're installed.

**Causes and solutions**:

1. **Package manager command detection**:
   ```bash
   # Santa checks if commands are in PATH
   # Verify package manager binaries are accessible
   which brew
   which cargo
   ```

2. **Package names don't match**:
   ```bash
   # Use -vv to see what Santa is checking
   santa status -vv

   # Update config with correct names
   ```

3. **Source not available on platform**:
   ```bash
   # List available sources
   santa sources list

   # Update config to use available sources
   ```

### Status Check is Slow

**Symptom**: `santa status` takes a long time to complete.

**Solutions**:

1. Santa checks each package manager sequentially. This is normal for many packages.

2. Enable caching (future feature - check latest release).

3. Limit check to specific source:
   ```bash
   # Check only from specific source
   santa config | grep sources
   ```

## Logging and Debugging

### Enable Debug Logging

Get detailed information about what Santa is doing:

```bash
# Info level
santa status -v

# Debug level
santa status -vv

# Trace level (very detailed)
santa status -vvv
```

Or use environment variable:

```bash
export RUST_LOG=santa=debug
santa status

export RUST_LOG=santa=trace
santa install
```

### Understanding Log Output

Logging levels:

- **ERROR**: Critical failures
- **WARN**: Issues that don't prevent operation
- **INFO** (`-v`): High-level operation information
- **DEBUG** (`-vv`): Detailed execution information
- **TRACE** (`-vvv`): Very detailed internal information

### Debug Commands

Useful commands for troubleshooting:

```bash
# Show full configuration
santa config --packages

# Show only configuration structure
santa config

# List sources and their origin
santa sources list

# Check with built-in config only
santa --builtin-only status

# Generate script without executing
santa install --output-dir ./debug-scripts

# Test with specific config file
SANTA_CONFIG=./test.ccl santa status
```

## Clean Slate

If Santa is behaving unexpectedly, start fresh:

### Reset User Configuration

```bash
# Backup existing config
cp ~/.config/santa/config.ccl ~/.config/santa/config.ccl.backup

# Remove user config (uses built-in defaults)
rm ~/.config/santa/config.ccl

# Test with defaults
santa status

# Restore if needed
mv ~/.config/santa/config.ccl.backup ~/.config/santa/config.ccl
```

### Clear Downloaded Sources

```bash
# Backup downloaded sources
cp -r ~/.local/share/santa/sources ~/.local/share/santa/sources.backup

# Remove downloaded sources
rm -rf ~/.local/share/santa/sources

# Re-download
santa sources update
```

### Complete Reset

```bash
# Backup everything
mkdir -p ~/santa-backup
cp -r ~/.config/santa ~/santa-backup/config
cp -r ~/.local/share/santa ~/santa-backup/data

# Remove all Santa data
rm -rf ~/.config/santa
rm -rf ~/.local/share/santa
rm -rf ~/.santa

# Start fresh
santa status
```

## Platform-Specific Issues

### macOS

**Homebrew not detected**:
```bash
# Verify Homebrew installation
which brew

# If not in PATH, add to shell profile
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
source ~/.zprofile
```

**Permission denied for scripts**:
```bash
# macOS requires explicit execute permission
chmod +x ~/.santa/scripts/*.sh
```

### Linux

**apt requires sudo**:
```bash
# Santa generates commands, doesn't handle sudo
# Review script and run with sudo if needed
cat ~/.santa/scripts/install_*.sh
sudo bash ~/.santa/scripts/install_*.sh
```

**Snap/Flatpak not supported**:

Santa currently doesn't support Snap or Flatpak. Use native package managers (apt, pacman, etc.) or cargo.

### Windows

**PowerShell execution policy**:
```powershell
# Allow script execution
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or run with bypass
powershell -ExecutionPolicy Bypass -File install_*.ps1
```

**Scoop not detected**:
```powershell
# Verify Scoop installation
scoop --version

# If not installed, install Scoop first
iwr -useb get.scoop.sh | iex
```

**Path issues**:

Windows uses different path separators. Santa handles this automatically, but if issues occur:

```powershell
# Check environment
echo $env:SANTA_CONFIG

# Use Windows-style paths
$env:SANTA_CONFIG = "C:\Users\username\config.ccl"
```

## Getting Help

If you're still experiencing issues:

1. **Check version**:
   ```bash
   santa --version
   ```

2. **Gather debug information**:
   ```bash
   santa status -vvv > debug.log 2>&1
   santa config --packages >> debug.log 2>&1
   ```

3. **Create minimal reproduction**:
   - Start with minimal config
   - Add complexity until issue appears
   - Document exact steps

4. **Report issue** on GitHub with:
   - Santa version
   - Operating system and version
   - Package manager versions
   - Configuration file (sanitized)
   - Debug logs
   - Steps to reproduce

## Common Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| `Failed to parse configuration` | CCL syntax error | Check indentation and `=` signs |
| `Package manager not found` | Binary not in PATH | Install package manager or update PATH |
| `Permission denied` | Insufficient permissions | Check file/directory permissions |
| `Unknown source` | Invalid source name | Use supported source names |
| `Failed to generate script` | Output directory issue | Check directory exists and is writable |

## Best Practices

To avoid issues:

1. **Start simple**: Begin with minimal configuration
2. **Use safe mode**: Review scripts before execution
3. **Test changes**: Use `santa config` to verify configuration
4. **Keep backups**: Save working configuration
5. **Update regularly**: Run `santa sources update` periodically
6. **Read logs**: Use `-v` to understand what's happening
7. **Check documentation**: Refer to [User Guide](user-guide.md) and [Configuration Guide](configuration.md)

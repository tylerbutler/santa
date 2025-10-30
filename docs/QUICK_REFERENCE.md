# Santa Quick Reference

**Fast lookup for common Santa operations**

## Installation

```bash
# From source
cargo install --git https://github.com/tylerbutler/santa

# Verify
santa --version
```

## Essential Commands

| Command | Description | Example |
|---------|-------------|---------|
| `santa status` | Show package status | `santa status` |
| `santa status --all` | Show all packages | `santa status --all` |
| `santa install` | Generate install scripts | `santa install` |
| `santa install --execute` | Direct execution | `santa install --execute` |
| `santa config` | Show configuration | `santa config` |
| `santa add` | Add package | `santa add neovim brew` |
| `santa completions` | Shell completions | `santa completions bash` |

## Global Flags

| Flag | Description | Example |
|------|-------------|---------|
| `-v, --verbose` | Increase logging | `santa status -vvv` |
| `-b, --builtin-only` | Use built-in config | `santa --builtin-only status` |
| `-x, --execute` | Direct execution | `santa install --execute` |
| `--format <FMT>` | Script format | `santa install --format powershell` |
| `--output-dir <DIR>` | Output directory | `santa install --output-dir ./scripts` |

## Configuration

### File Locations (in order)

1. `$SANTA_CONFIG_PATH`
2. `~/.config/santa/config.yaml`
3. Built-in defaults

### CCL Configuration

```ccl
sources = [brew, cargo, apt]

packages = {
  bat: "Better cat"
  ripgrep: "Fast grep"
  git: "Version control"
}

package_names = {
  brew: {
    rust: "rustup"
  }
}
```

### Environment Variables

```bash
# Core
export SANTA_LOG_LEVEL="debug"          # trace, debug, info, warn, error
export SANTA_CONFIG_PATH="/path"        # Custom config path
export SANTA_BUILTIN_ONLY="true"        # Use built-in only

# Sources & Packages
export SANTA_SOURCES="brew,cargo"       # Override sources
export SANTA_PACKAGES="git,rust"        # Override packages

# Performance
export SANTA_CACHE_TTL_SECONDS="600"    # Cache TTL (default: 300)
export SANTA_CACHE_SIZE="2000"          # Max entries (default: 1000)

# Features
export SANTA_HOT_RELOAD="true"          # Hot-reload config
export SANTA_VERBOSE="2"                # Verbosity (0-3)
```

## Package Managers

| Manager | Platforms | Source Name |
|---------|-----------|-------------|
| Homebrew | macOS, Linux | `brew` |
| Cargo | All | `cargo` |
| APT | Debian, Ubuntu | `apt` |
| Pacman | Arch Linux | `pacman` |
| Scoop | Windows | `scoop` |
| Nix | All | `nix` |

## Common Workflows

### Daily Usage

```bash
# Check what's missing
santa status

# Generate scripts
santa install

# Review and run
./install-brew.sh
./install-cargo.sh
```

### CI/CD Pipeline

```bash
# In .github/workflows/setup.yml
- name: Setup tools
  run: |
    santa install --builtin-only
    chmod +x *.sh
    ./install-brew.sh
```

### Dotfiles Integration

```bash
# In dotfiles setup script
santa install --builtin-only
find . -name "install-*.sh" -exec {} \;
```

### Development Setup

```bash
# Clone project
git clone https://github.com/tylerbutler/santa
cd santa

# Install dev tools
just setup

# Run checks
just check-quick

# Run tests
just test
```

## Troubleshooting

### Common Issues

```bash
# Package manager not found
# → Install the package manager first

# Config not found
santa status --builtin-only

# Permission denied
sudo ./install-apt.sh

# Cache issues
SANTA_CACHE_TTL_SECONDS=60 santa status
```

### Debug Mode

```bash
# Maximum verbosity
SANTA_LOG_LEVEL=trace santa status

# Or with flags
santa status -vvv
```

## Shell Completions

```bash
# Bash
santa completions bash >> ~/.bashrc
source ~/.bashrc

# Zsh
santa completions zsh >> ~/.zshrc
source ~/.zshrc

# Fish
santa completions fish >> ~/.config/fish/config.fish
```

## Development

### Build & Test

```bash
just build              # Build debug
just build-release      # Build release
just test               # Run tests
just test-fast          # Fast tests
just check-quick        # Quick checks
just check-all          # Full checks
```

### Code Quality

```bash
just lint               # Clippy
just format             # Format code
just audit              # Security audit
just fix                # Auto-fix issues
```

### Documentation

```bash
just docs               # Generate docs
just docs-check         # Check docs
cargo doc --open        # Open docs
```

## Library Usage

### Basic Example

```rust
use santa::{SantaConfig, SantaData, sources::PackageCache};
use santa::commands::status_command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = SantaConfig::default();
    let data = SantaData::default();
    let cache = PackageCache::new();

    status_command(&mut config, &data, cache, &false).await?;
    Ok(())
}
```

### Add Dependency

```toml
[dependencies]
santa = { git = "https://github.com/tylerbutler/santa" }
tokio = { version = "1", features = ["full"] }
```

## Security

### Safe Mode (Default)

```bash
# Generates scripts for review
santa install

# Review scripts
cat install-brew.sh

# Execute manually
./install-brew.sh
```

### Direct Execution

```bash
# Requires confirmation
santa install --execute

# Environment variable
export SANTA_EXECUTE=true
santa install
```

## Performance

```bash
# Cache tuning
export SANTA_CACHE_SIZE=5000
export SANTA_CACHE_TTL_SECONDS=1800

# Verbose timing
santa status -vvv
```

## Resources

- **Documentation**: `/docs/USER_GUIDE.md`
- **API Reference**: `/docs/API.md`
- **Repository**: https://github.com/tylerbutler/santa
- **Issues**: https://github.com/tylerbutler/santa/issues

---

*Last updated: September 2024*

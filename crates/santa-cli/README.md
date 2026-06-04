# Santa

Santa is a command-line tool that installs and tracks a common set of packages
across different platforms and package managers. You keep one list of the tools
you use, and Santa installs them from whichever package manager is available.

[![Build Status](https://github.com/tylerbutler/santa/actions/workflows/test.yml/badge.svg)](https://github.com/tylerbutler/santa/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/tylerbutler/santa/branch/main/graph/badge.svg)](https://codecov.io/gh/tylerbutler/santa)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Why use it

If you prefer tools like `ripgrep`, `bat`, and `fd` that aren't installed by
default, setting them up on a new machine is tedious, especially when the right
package manager varies by platform. Santa lets you keep one list of packages and
installs each from an available source.

By default, `santa install` generates a script for you to review rather than
running commands directly.

## Installation

### From releases

Download a pre-built binary from the
[releases page](https://github.com/tylerbutler/santa/releases):

```bash
# Linux (x64)
curl -L https://github.com/tylerbutler/santa/releases/latest/download/santa-linux-x64.tar.gz | tar xz

# macOS (Apple Silicon)
curl -L https://github.com/tylerbutler/santa/releases/latest/download/santa-macos-aarch64.tar.gz | tar xz
```

### From source

```bash
cargo install santa-cli
```

## Quick start

```bash
# See which tracked packages are installed or missing
santa status

# Add a package to your tracking list
santa add ripgrep cargo

# Generate an installation script for missing packages
santa install

# Review and run the generated script
sh ~/.santa/scripts/install_*.sh
```

## Common commands

| Command | Description |
|---------|-------------|
| `santa status` | Show missing packages (`--all` for everything) |
| `santa install [source]` | Generate an install script (`-x` to run directly) |
| `santa add <package> <source>` | Add a package to your tracking list |
| `santa config` | Show the current configuration |
| `santa sources list` | List available package sources |
| `santa sources update` | Download the latest source definitions |
| `santa completions <shell>` | Generate shell completions |

See the [CLI Reference](../../docs/cli-reference.md) for the full list of
commands and options.

## Configuration

Santa reads a CCL (Categorical Configuration Language) file, by default at
`~/.config/santa/config.ccl`:

```ccl
/= Package sources in order of preference
sources =
  = brew
  = cargo
  = apt

/= Packages to track
packages =
  = bat
  = ripgrep
  = fd
```

See the [Configuration Guide](../../docs/configuration.md) for the full format
and options.

## Supported package managers

| Package manager | Platforms |
|-----------------|-----------|
| Homebrew | macOS, Linux |
| Cargo | All |
| APT | Debian, Ubuntu |
| Pacman | Arch Linux |
| AUR | Arch Linux |
| Scoop | Windows |
| Nix | All |

## Documentation

- [User Guide](../../docs/user-guide.md)
- [Configuration Guide](../../docs/configuration.md)
- [Troubleshooting](../../docs/troubleshooting.md)
- [CLI Reference](../../docs/cli-reference.md)

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](../../CONTRIBUTING.md) and
[DEVELOPMENT.md](../../DEVELOPMENT.md) in the main repository.

## License

Licensed under the [MIT License](LICENSE).

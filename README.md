# Santa Workspace

Santa is a package manager meta-tool written in Rust. It helps you install and
track a common set of packages across different platforms and package managers
with a single tool.

[![Build Status](https://github.com/tylerbutler/santa/actions/workflows/test.yml/badge.svg)](https://github.com/tylerbutler/santa/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/tylerbutler/santa/branch/main/graph/badge.svg)](https://codecov.io/gh/tylerbutler/santa)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Workspace structure

This is a Cargo workspace with three crates:

### [santa-cli](crates/santa-cli)

The Santa command-line application. It checks and installs packages across
Homebrew, Cargo, APT, Pacman, AUR, Scoop, and Nix.

```bash
# Install Santa
cargo install santa-cli

# Check package status
santa status

# Generate an installation script for missing packages
santa install

# Review and run the generated script
sh ~/.santa/scripts/install_*.sh
```

By default, `santa install` writes a script for you to review rather than
running commands directly. See the [User Guide](docs/user-guide.md) for details.

### [santa-data](crates/santa-data)

Data models, schemas, and the CCL configuration parser used by Santa. It can be
reused by other tools that need to work with Santa's configuration format.

```rust
use santa_data::parser::parse_ccl_config;

let config = parse_ccl_config(ccl_string)?;
```

### [sickle](crates/sickle)

A Rust parser for CCL (Categorical Configuration Language) with Serde support.
It is published independently and has no dependency on Santa.

```rust
use sickle::{parse, from_str};

let model = parse(ccl_string)?;
let config: MyConfig = from_str(ccl_string)?;
```

## Documentation

- User docs: [User Guide](docs/user-guide.md) · [Configuration](docs/configuration.md) · [Troubleshooting](docs/troubleshooting.md) · [CLI Reference](docs/cli-reference.md)
- API docs: [docs.rs/santa-data](https://docs.rs/santa-data) · [docs.rs/sickle](https://docs.rs/sickle)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Contributing: [CONTRIBUTING.md](CONTRIBUTING.md) · [DEVELOPMENT.md](DEVELOPMENT.md)

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

## Development

Santa uses [`just`](https://github.com/casey/just) to run development tasks.

```bash
git clone https://github.com/tylerbutler/santa.git
cd santa

# Install development tools
just setup

# Build, test, and lint
just build
just test
just lint
```

Run `just` with no arguments to list the available tasks. See
[DEVELOPMENT.md](DEVELOPMENT.md) for the full workflow.

## License

All crates in this workspace are licensed under the [MIT License](LICENSE).

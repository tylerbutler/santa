# Contributing to Santa

Thank you for your interest in contributing to Santa! This guide will help you get started with development.

## Development Setup

### Prerequisites

1. **Rust** (1.80 or later): Install from [rustup.rs](https://rustup.rs/)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Just** task runner:
   ```bash
   cargo install just
   ```

3. **Git**:
   ```bash
   # Verify git is installed
   git --version
   ```

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/tylerbutler/santa.git
cd santa

# Install development tools
just setup

# Build the project
just build

# Run tests to verify setup
just test
```

## Development Workflow

### Quick Checks

Run quick validation before committing:

```bash
# Format, lint, and run quick tests
just check-quick
```

### Complete Pre-Commit Checks

Run all checks before submitting a PR:

```bash
# Format, lint, tests, and security audit
just check-all
```

### Building

```bash
# Build all workspace crates
just build

# Build in release mode
cargo build --release

# Build specific crate
cd crates/santa-cli && cargo build
```

### Testing

```bash
# Run all tests
just test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific crate
cd crates/santa-cli && cargo test

# Fast parallel testing
just test-fast

# Run tests with coverage
just test-coverage
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without changing files
cargo fmt -- --check

# Run clippy linter
just lint

# Fix clippy warnings automatically
just fix

# Check code style
just check-style
```

### Documentation

```bash
# Generate and open documentation
just docs

# Build documentation without opening
cargo doc --no-deps

# Check documentation links
cargo doc --no-deps --document-private-items
```

## Code Standards

### General Principles

1. **No `unwrap()` or `todo!()` in production code**
   - Use proper error handling with `Result` and `Option`
   - Use `expect()` only in tests or with clear justification

2. **Comprehensive error handling**
   - Use `anyhow::Context` to add error context
   - Use `thiserror` for custom error types
   - Provide user-friendly error messages

3. **Security-first approach**
   - All user inputs must be sanitized
   - Use `shell-escape` for command arguments
   - Prefer script generation over direct execution

4. **Strong typing**
   - Use builder patterns for complex structures
   - Validate inputs at construction time
   - Leverage the type system for correctness

### Code Style

Follow standard Rust conventions:

- Use `cargo fmt` for formatting (enforced in CI)
- Follow Rust naming conventions (snake_case for functions, CamelCase for types)
- Keep functions focused and small
- Write self-documenting code with clear variable names
- Add comments for complex logic, not obvious code

### Error Handling Patterns

**Good:**
```rust
use anyhow::{Context, Result};

fn read_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path)
        .context(format!("Failed to read config from {}", path.display()))?;

    parse_config(&contents)
        .context("Failed to parse configuration")
}
```

**Bad:**
```rust
fn read_config(path: &Path) -> Config {
    let contents = std::fs::read_to_string(path).unwrap(); // ❌ No unwrap
    parse_config(&contents).unwrap() // ❌ No context
}
```

### Async Code

- Use `tokio::process::Command` for subprocess execution
- Use `async`/`await` consistently
- Handle timeouts for long-running operations
- Use `tokio::sync::RwLock` for shared state

### Security

- **Input validation**: All user inputs must be validated
- **Command injection prevention**: Use `shell-escape` for arguments
- **Safe-by-default**: Script generation mode is default
- **No blind execution**: Review generated scripts before execution

## Testing Requirements

### Test Categories

1. **Unit tests**: Test individual functions and modules
2. **Integration tests**: Test end-to-end workflows
3. **Property-based tests**: Use `proptest` for parsers and data structures
4. **Security tests**: Validate sanitization and injection prevention

### Writing Tests

**Unit tests** go in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let input = "key = value";
        let result = parse(input).unwrap();
        assert_eq!(result.get("key"), Some(&"value".to_string()));
    }
}
```

**Integration tests** go in `tests/` directory:

```rust
// tests/integration_test.rs
use santa::configuration::SantaConfig;

#[test]
fn test_config_loading() {
    let config = SantaConfig::load_default().unwrap();
    assert!(!config.sources.is_empty());
}
```

### Test Coverage

- Aim for >80% code coverage
- All public APIs must have tests
- Critical paths must have comprehensive tests
- Edge cases should be tested

Run coverage:

```bash
just test-coverage
```

## Benchmarking

Santa uses Criterion for benchmarking:

```bash
# Run all benchmarks
just bench

# Save baseline for comparison
just bench-baseline my-feature

# Compare against baseline
just bench-compare my-feature

# Run specific benchmark
cargo bench --bench my_benchmark
```

Add benchmarks for:
- Performance-critical code paths
- Parsing operations
- Concurrent operations
- Cache performance

## Pull Request Process

### Before Submitting

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**:
   - Write code following standards above
   - Add tests for new functionality
   - Update documentation as needed

3. **Run all checks**:
   ```bash
   just check-all
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

### Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Test additions or changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Build/tooling changes

Examples:
```bash
git commit -m "feat: add support for DNF package manager"
git commit -m "fix: handle empty configuration files gracefully"
git commit -m "docs: update configuration guide with examples"
git commit -m "test: add integration tests for script generation"
```

### Submitting PR

1. **Push your branch**:
   ```bash
   git push -u origin feature/my-feature
   ```

2. **Create PR** on GitHub with:
   - Clear title describing the change
   - Description of what changed and why
   - Reference to related issues (if any)
   - Screenshots/examples if applicable

3. **Address review feedback**:
   - Make requested changes
   - Run `just check-all` again
   - Push updates to the same branch

## Project Structure

```
santa/
├── crates/
│   ├── santa-cli/         # Main CLI application
│   │   ├── src/
│   │   │   ├── main.rs    # CLI entry point
│   │   │   ├── commands/  # Command implementations
│   │   │   └── ...
│   │   ├── tests/         # Integration tests
│   │   └── Cargo.toml
│   ├── santa-data/        # Data models and configuration
│   │   ├── src/
│   │   └── Cargo.toml
│   └── sickle/           # CCL parser library
│       ├── src/
│       ├── tests/
│       └── Cargo.toml
├── data/                 # Package data and definitions
├── templates/            # Script generation templates
├── docs/                 # Documentation
├── scripts/              # Development scripts
├── justfile              # Task runner
├── Cargo.toml            # Workspace configuration
└── README.md
```

## Common Development Tasks

### Adding a New Command

1. Define command in `crates/santa-cli/src/main.rs`:
   ```rust
   #[derive(Subcommand)]
   enum Commands {
       /// Your new command
       MyCommand {
           #[clap(short, long)]
           option: String,
       },
   }
   ```

2. Implement handler in `crates/santa-cli/src/commands/`:
   ```rust
   pub async fn handle_my_command(option: String) -> Result<()> {
       // Implementation
       Ok(())
   }
   ```

3. Add tests in `tests/`:
   ```rust
   #[test]
   fn test_my_command() {
       // Test implementation
   }
   ```

4. Update documentation in `docs/user-guide.md`

### Adding a New Package Manager

1. Implement source in `crates/santa-data/src/sources/`
2. Add to source registry
3. Add tests for detection and installation
4. Update documentation

### Modifying CCL Parser

1. Make changes in `crates/sickle/`
2. Add comprehensive tests
3. Run property-based tests: `cargo test --package sickle`
4. Update examples if syntax changes

## CI/CD

All pull requests run through CI:

- **Multi-platform testing**: Linux, macOS, Windows
- **Test suite**: All tests must pass
- **Code coverage**: Coverage must not decrease
- **Linting**: Clippy must pass with no warnings
- **Formatting**: Code must be formatted with `cargo fmt`
- **Security audit**: `cargo audit` must pass

You can run equivalent checks locally:

```bash
# Run all CI checks
just ci

# Platform-specific checks
just ci-linux
just ci-macos
just ci-windows
```

## Getting Help

- **Questions**: Open a [GitHub Discussion](https://github.com/tylerbutler/santa/discussions)
- **Bugs**: Open a [GitHub Issue](https://github.com/tylerbutler/santa/issues)
- **Security**: Email security concerns to maintainers privately

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on what's best for the project
- Follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)

## License

By contributing to Santa, you agree that your contributions will be licensed under the MIT License.

## Recognition

Contributors are recognized in:
- GitHub contributors list
- Release notes for their contributions
- Project acknowledgments

Thank you for contributing to Santa!

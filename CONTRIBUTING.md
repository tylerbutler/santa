# Contributing to Santa

Thank you for your interest in contributing to Santa! This guide covers the contribution process and code standards.

For development setup and workflows, see [DEVELOPMENT.md](DEVELOPMENT.md).

## Pull Request Process

### Before Submitting

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**:
   - Write code following the standards below
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

3. **Address review feedback**:
   - Make requested changes
   - Run `just check-all` again
   - Push updates to the same branch

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
    let contents = std::fs::read_to_string(path).unwrap();
    parse_config(&contents).unwrap()
}
```

## Testing Requirements

### Test Categories

1. **Unit tests**: Test individual functions and modules
2. **Integration tests**: Test end-to-end workflows
3. **Property-based tests**: Use `proptest` for parsers and data structures
4. **Security tests**: Validate sanitization and injection prevention

### Test Coverage

- Aim for >80% code coverage
- All public APIs must have tests
- Critical paths must have comprehensive tests
- Edge cases should be tested

## CI/CD

All pull requests run through CI:

- **Multi-platform testing**: Linux, macOS, Windows
- **Test suite**: All tests must pass
- **Code coverage**: Coverage must not decrease
- **Linting**: Clippy must pass with no warnings
- **Formatting**: Code must be formatted with `cargo fmt`
- **Security audit**: `cargo audit` must pass

Run equivalent checks locally:

```bash
just ci
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

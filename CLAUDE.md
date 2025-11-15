# Santa Package Manager - Claude Code Configuration

## Project Overview
Santa is a Rust-based package manager meta-tool that provides unified interfaces across different package managers. The project has evolved to focus on safe script generation rather than direct command execution, with comprehensive CCL-based configuration and robust error handling.

## Current Architecture (September 2024)
The project has implemented significant architectural improvements:

### Script Generation Model
- **Safe-by-default execution**: Uses `ScriptGenerator` to create platform-specific scripts
- **Execution modes**: `Safe` (generate scripts) vs `Execute` (direct execution) 
- **Multi-platform support**: Shell scripts (.sh), PowerShell (.ps1), and Batch (.bat)
- **Template-driven**: Uses Tera templating engine for flexible script generation

### Configuration System
- **CCL format**: Modern configuration using Categorical Configuration Language
- **Migration support**: Transparent YAML-to-CCL migration for legacy configs
- **Hot-reloading**: Real-time configuration updates with file system watchers

## Development Guidelines

### Security First
- All user inputs are properly sanitized using `shell-escape` crate
- Script generation prevents command injection by design
- Safe-by-default execution mode requires explicit opt-in for direct command execution
- Never trust package names or user-provided strings

### Error Handling Standards
- Unified `SantaError` types with `thiserror` for structured error handling
- Contextual error information with `anyhow` for error chaining
- Graceful failure handling and user-friendly error messages

### Async Patterns
- Standardized on `tokio::process::Command` for all subprocess execution
- Consistent async/await patterns throughout codebase
- Proper timeout handling for long-running operations
- Use `tokio::sync::RwLock` for shared state in async contexts

### Code Quality
- Minimize cloning in hot paths
- Use iterators over owned collections where possible
- Follow trait-based design patterns
- Maintain comprehensive test coverage

## Key Files & Responsibilities

### Core Architecture
- `src/main.rs` - CLI entry point with clap derive macros
- `src/lib.rs` - Library exports and public API
- `src/configuration/` - CCL config management with hot-reloading
- `src/sources.rs` - Package source abstractions with security improvements
- `src/commands.rs` - Command implementations using script generation
- `src/script_generator.rs` - Safe script generation with Tera templates
- `src/migration/` - YAML-to-CCL configuration migration

### Data & Models  
- `src/data/` - Data models, schemas, and platform detection
- `src/errors.rs` - Unified error types and handling
- `src/traits.rs` - Core trait definitions for package managers

### Testing
- `tests/` - Comprehensive integration tests including config hot-reload
- `tests/security_tests.rs` - Security-focused tests
- `tests/property_tests.rs` - Property-based testing with proptest
- Unit tests - Embedded throughout source files
- `benches/` - Performance benchmarks

### Scripts & Tooling
- `scripts/` - Python-based package collection and analysis tools
- `justfile` - Task runner with build, test, and deployment commands

## Current State (September 2024)
- **Lines of Code**: ~8,100 Rust lines
- **Architecture**: Mature script-generation model with security focus
- **Configuration**: CCL-based with migration support
- **Dependencies**: Clean, well-documented dependency tree
- **Security**: Command injection vulnerabilities resolved through script generation
- **Test Coverage**: Comprehensive unit, integration, and property-based tests

## Current Focus Areas
1. **Package collection automation** - Scripts to identify installable packages
2. **Template system refinement** - Enhanced script generation capabilities  
3. **Performance optimization** - Benchmarking and caching improvements
4. **Cross-platform compatibility** - Windows, macOS, and Linux support

## Development Workflow
When working on this project:
1. **Review recent changes** - Check git history and current branch state
2. **Run tests first** - Execute `cargo test` to ensure baseline functionality
3. **Follow security patterns** - Use script generation over direct execution
4. **Maintain consistency** - Follow established async and error handling patterns
5. **Update documentation** - Keep CLAUDE.md current with architectural changes

## Testing Strategy
- `cargo test` - Comprehensive unit and integration tests
- `cargo check` - Fast compilation and type checking
- `just test` - Run full test suite via justfile
- Security tests validate script generation safety
- Property-based tests with proptest for edge cases
- Performance benchmarks in `benches/` directory

## Build & Quality Checks
- `cargo clippy` - Linting with custom rules in `deny.toml`
- `cargo fmt` - Code formatting consistency
- `just build` - Full build process via justfile
- `cargo audit` - Dependency vulnerability scanning
- Performance profiling for subprocess operations

## Key Dependencies
- **Script Generation**: `tera` (templating), `shell-escape` (safety)
- **Configuration**: `serde_ccl` (parsing), `config` (management)
- **CLI**: `clap` (arguments), `dialoguer` (interactive prompts)
- **Async**: `tokio` (runtime), `futures` (utilities)
- **Error Handling**: `anyhow` (context), `thiserror` (structured errors)
- **Testing**: `rstest` (fixtures), `proptest` (property-based), `mockall` (mocking)

---
*This configuration reflects the current state as of September 2024. Update as new architectural patterns emerge.*
- The CCL format is documented at ccl.tylerbutler.com.
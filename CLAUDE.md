# Santa Package Manager - Claude Code Configuration

## Project Overview
Santa is a Rust-based package manager meta-tool that provides unified interfaces across different package managers. This project follows modern Rust practices with async/await, comprehensive testing, and modular architecture.

## Architecture & Improvement Plans
**Primary Reference**: `ARCHITECTURE_IMPROVEMENT_PLAN.md`

This file contains a comprehensive architectural review and improvement roadmap covering:
- 🚨 Critical security vulnerabilities (command injection)
- 🔥 High-priority architectural inconsistencies  
- ⚡ Performance optimizations and code quality improvements
- 📈 Long-term maintainability enhancements

**Start Here**: Always consult the Architecture Improvement Plan before making significant changes to understand:
- Current architectural state and issues
- Prioritized improvement recommendations
- Implementation timeline and success metrics
- Specific file locations and code patterns to address

## Development Guidelines

### Security First
- All user inputs must be properly sanitized (see Phase 1 in improvement plan)
- Use `shell-escape` crate for command execution
- Never trust package names or user-provided strings

### Error Handling Standards
- Use unified `SantaError` types (defined in improvement plan Phase 2)
- Provide contextual error information
- Avoid silent failures

### Async Patterns
- Use `tokio::sync::RwLock` instead of `std::sync::Mutex` in async contexts
- Standardize on `tokio::process::Command` for subprocess execution
- Follow consistent async/await patterns

### Code Quality
- Minimize cloning in hot paths
- Use iterators over owned collections where possible
- Follow trait-based design patterns
- Maintain comprehensive test coverage

## Key Files & Responsibilities

### Core Architecture
- `src/main.rs` - CLI entry point
- `src/lib.rs` - Library exports and public API
- `src/configuration/` - Config management with hot-reloading
- `src/sources.rs` - Package source abstractions ⚠️ Contains security vulnerabilities
- `src/commands.rs` - Command implementations ⚠️ Async consistency issues

### Data & Models  
- `src/data/` - Data models and platform detection
- `src/traits.rs` - Trait definitions (needs expansion)

### Testing
- `tests/` - Integration tests
- Unit tests - Embedded in source files

## Current State (as of Aug 30, 2025)
- **Lines of Code**: ~5,650
- **Critical Issues**: 2 command injection vulnerabilities
- **Architecture**: Solid foundation with consistency issues
- **Dependencies**: Some redundancy (subprocess + tokio::process)
- **Test Coverage**: Good unit tests, needs integration test expansion

## Immediate Priorities
1. **Fix security vulnerabilities** in `src/sources.rs` (command injection)
2. **Implement unified error handling** across all modules
3. **Standardize async patterns** for better performance
4. **Clean up dependencies** and reduce redundancy

## Future Sessions
When working on this project:
1. **Always start** by reviewing `ARCHITECTURE_IMPROVEMENT_PLAN.md`
2. **Check current phase** of implementation timeline
3. **Prioritize security fixes** before feature development
4. **Follow established patterns** for consistency
5. **Update the plan** as implementation progresses

## Testing Strategy
- Run `cargo test` for unit tests
- Use `cargo check` for quick compilation checks  
- Integration tests focus on command execution and config loading
- Security tests prevent regression of vulnerability fixes

## Build & Quality Checks
- `cargo clippy` for linting
- `cargo fmt` for formatting
- Consider adding `cargo audit` for security scanning
- Profile performance with typical workloads

---
*This configuration file should be updated as the architecture improvement plan is implemented and new patterns emerge.*
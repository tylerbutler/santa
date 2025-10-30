# Santa Documentation

**Complete documentation for Santa, the modern package manager meta-tool**

## Documentation Index

### User Documentation

- **[User Guide](./USER_GUIDE.md)** - Complete guide for end users
  - Installation instructions
  - Configuration
  - All commands explained
  - Advanced usage patterns
  - Troubleshooting guide
  - FAQ

- **[Quick Reference](./QUICK_REFERENCE.md)** - Fast lookup guide
  - Command cheat sheet
  - Configuration quick reference
  - Common workflows
  - Troubleshooting shortcuts

### Developer Documentation

- **[API Documentation](./API.md)** - Library usage guide
  - Getting started with the Santa library
  - Core concepts and APIs
  - Configuration API
  - Package sources
  - Script generation
  - Caching system
  - Error handling
  - Code examples

- **[Rust API Docs](https://docs.rs/santa)** - Generated API documentation
  - Run `cargo doc --open` to view locally

### Project Documentation

- **[README](../README.md)** - Project overview
  - Features
  - Quick start
  - Development setup
  - CI/CD information

- **[CLAUDE.md](../CLAUDE.md)** - Project architecture and guidelines
  - Current architecture
  - Development guidelines
  - Code quality standards
  - Testing strategy

## Getting Started

### For Users

1. Start with the [User Guide](./USER_GUIDE.md) for comprehensive instructions
2. Use the [Quick Reference](./QUICK_REFERENCE.md) for day-to-day operations
3. Check the [FAQ](./USER_GUIDE.md#faq) for common questions

### For Developers

1. Read the [API Documentation](./API.md) to understand library usage
2. Check the [README](../README.md) for development setup
3. Review [CLAUDE.md](../CLAUDE.md) for architecture and guidelines
4. Generate local API docs with `cargo doc --open`

### For Contributors

1. Read the [README](../README.md) development section
2. Review [CLAUDE.md](../CLAUDE.md) for code standards
3. Check the [User Guide](./USER_GUIDE.md) to understand user expectations
4. Run `just check-all` before submitting PRs

## Documentation Standards

### For Maintainers

When updating documentation:

1. **User Guide** - Update for new features, commands, or workflows
2. **API Documentation** - Update for library API changes
3. **Quick Reference** - Keep commands and examples current
4. **Code Documentation** - Maintain Rustdoc comments for all public APIs
5. **README** - Update version numbers, features, and links

### Documentation Requirements

- **Public APIs**: Must have Rustdoc comments with examples
- **New features**: Must update User Guide and Quick Reference
- **Breaking changes**: Must update API Documentation and migration guide
- **CLI changes**: Must update User Guide command reference

## Building Documentation

### User Documentation

All markdown files in this directory are ready to view:

```bash
# View in your browser or editor
open docs/USER_GUIDE.md
```

### API Documentation

Generate Rust API documentation:

```bash
# Generate and open
cargo doc --open

# Generate without opening
cargo doc

# Include private items
cargo doc --document-private-items
```

### Full Documentation Site

```bash
# Check all docs compile correctly
just docs-check

# Generate everything
just docs
```

## Documentation Coverage

- ✅ **User Guide** - Complete with examples, troubleshooting, FAQ
- ✅ **API Documentation** - Full library usage guide with examples
- ✅ **Quick Reference** - Commands, config, workflows
- ✅ **Code Documentation** - Rustdoc comments on public APIs
- ✅ **README** - Project overview and quick start
- ✅ **Architecture** - CLAUDE.md with guidelines and standards

## Contributing to Documentation

### Improving Existing Docs

1. Fix typos, unclear explanations, or outdated information
2. Add examples where missing
3. Improve organization and readability
4. Add troubleshooting entries

### Adding New Documentation

1. User-facing features → Update User Guide + Quick Reference
2. API changes → Update API Documentation + Rustdoc comments
3. Architecture changes → Update CLAUDE.md
4. Development processes → Update README

### Documentation Style Guide

- **Clear and concise** - Avoid unnecessary complexity
- **Example-driven** - Show, don't just tell
- **User-focused** - Write for the reader's needs
- **Well-organized** - Use clear headings and sections
- **Up-to-date** - Keep synchronized with code changes

## Help & Support

### Finding Information

1. **Commands** → [Quick Reference](./QUICK_REFERENCE.md)
2. **Features** → [User Guide](./USER_GUIDE.md)
3. **Library** → [API Documentation](./API.md)
4. **Development** → [README](../README.md) + [CLAUDE.md](../CLAUDE.md)

### Getting Help

- **User questions** → [GitHub Discussions](https://github.com/tylerbutler/santa/discussions)
- **Bug reports** → [GitHub Issues](https://github.com/tylerbutler/santa/issues)
- **Feature requests** → [GitHub Issues](https://github.com/tylerbutler/santa/issues)
- **Security issues** → See [SECURITY.md](../SECURITY.md)

---

## Quick Links

- [Project Repository](https://github.com/tylerbutler/santa)
- [Issue Tracker](https://github.com/tylerbutler/santa/issues)
- [Discussions](https://github.com/tylerbutler/santa/discussions)
- [Releases](https://github.com/tylerbutler/santa/releases)
- [Contributing Guide](../CONTRIBUTING.md)

---

*Documentation maintained by the Santa project team*

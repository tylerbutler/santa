# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in Santa Package Manager, please report it to:

**Email**: [security contact email - update with actual contact]
**GitHub Security Advisories**: https://github.com/tylerbutler/santa/security/advisories

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Varies by severity (critical issues prioritized)

## Supported Versions

Security updates are provided for:

- Latest release (v1.0-beta and above)
- Main branch (development)

Older versions may receive critical security patches on a case-by-case basis.

## Security Measures

Santa implements multiple security measures to protect against common threats:

### Command Injection Prevention

- **Shell Escaping**: All user inputs and package names are properly escaped using the `shell-escape` crate
- **Script Generation**: Safe-by-default execution mode generates scripts rather than executing commands directly
- **Input Sanitization**: Package names are sanitized to remove dangerous characters and null bytes

### Input Validation

- **Type-safe Parsing**: Configuration files are parsed with strict type checking
- **Path Traversal Protection**: Configuration and file paths are validated
- **Unicode Normalization**: Dangerous Unicode characters are detected and sanitized

### Safe Defaults

- **Script Generation Mode**: Default execution mode generates reviewable scripts
- **Explicit Execute Flag**: Direct command execution requires explicit `--execute` flag
- **Builtin Configurations**: Default configurations are bundled and trusted

## Security Testing

Run security-focused tests:

```bash
# Run security tests
cargo test security

# Run property-based tests with random inputs
cargo test property

# Run all tests including E2E
cargo test
```

## Known Security Considerations

### Execution Modes

- **Safe Mode (default)**: Generates scripts for user review before execution
- **Execute Mode (`--execute`)**: Directly executes package manager commands - use with caution

### Configuration Files

- **Custom Configurations**: Only load configuration files from trusted sources
- **CCL Format**: Configuration files are parsed using the CCL (Categorical Configuration Language) format
- **Validation**: All configuration values are validated before use

### Package Manager Trust

- Santa relies on the security of underlying package managers (apt, brew, cargo, etc.)
- Package authenticity and integrity are managed by the respective package managers
- Review generated scripts before execution to verify intended operations

## Best Practices for Users

1. **Review Generated Scripts**: Always review scripts before execution
2. **Use Builtin Configurations**: Prefer `--builtin-only` flag when testing
3. **Validate Custom Configs**: Thoroughly review custom configuration files
4. **Keep Updated**: Use the latest version for security patches
5. **Report Issues**: Report security concerns promptly through proper channels

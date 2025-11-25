#!/usr/bin/env just --justfile

# Santa Package Manager - Development Commands
#
# This justfile provides convenient commands for development, testing, and deployment.
# Install just: https://github.com/casey/just#installation

export RUST_BACKTRACE := "1"

# Common aliases for faster development
alias b := build
alias br := build-release
alias r := release
alias t := test
alias tf := test-fast
alias l := lint
alias f := fix
alias c := check-quick

# Default recipe - shows available commands
default:
    @just --list

# Development Commands
# ===================

# Install all development dependencies
setup:
    @echo "🔧 Setting up development environment..."
    cargo install cargo-udeps --locked || echo "cargo-udeps already installed"
    cargo install cargo-nextest --locked || echo "cargo-nextest already installed"
    cargo install cargo-llvm-cov --locked || echo "cargo-llvm-cov already installed"
    cargo install cargo-audit --locked || echo "cargo-audit already installed"
    cargo install cargo-deny --locked || echo "cargo-deny already installed"
    cargo install cargo-watch --locked || echo "cargo-watch already installed"
    cargo install cargo-outdated --locked || echo "cargo-outdated already installed"
    cargo install cargo-dist --locked || echo "cargo-dist already installed"
    @echo "✅ Development setup complete!"

# Build the project in debug mode
build *ARGS='':
    @echo "🔨 Building santa (debug)..."
    cargo build {{ARGS}}

# Build the project in release mode
build-release *ARGS='':
    @echo "🔨 Building santa (release)..."
    cargo build --release {{ARGS}}

# Build for CI with specific target
ci-build TARGET='x86_64-unknown-linux-gnu' *ARGS='':
    @echo "🔨 Building for CI target: {{TARGET}}"
    cargo build --locked --release --target {{TARGET}} {{ARGS}}

# Cross-compile build (requires cross)
cbuild target='x86_64-unknown-linux-gnu' *ARGS='':
    @echo "🔨 Cross-building for: {{target}}"
    cross build --locked --release --target {{target}} {{ARGS}}

# Build for all supported targets
build-all:
    @echo "🔨 Building for all targets..."
    cargo build --target x86_64-unknown-linux-gnu
    cargo build --target aarch64-unknown-linux-gnu
    cargo build --target x86_64-apple-darwin
    cargo build --target aarch64-apple-darwin
    cargo build --target x86_64-pc-windows-gnu

# Testing Commands
# ================

# Run all tests with cargo test
test *ARGS='':
    @echo "🧪 Running tests..."
    cargo test {{ARGS}}

# Run tests with nextest (faster parallel execution)
test-fast *ARGS='':
    @echo "🧪 Running tests with nextest..."
    cargo nextest run {{ARGS}}

# Run only unit tests
test-unit:
    @echo "🧪 Running unit tests..."
    cargo test --lib

# Run only integration tests
test-integration:
    @echo "🧪 Running integration tests..."
    cargo test --test '*'

# Run tests with coverage reporting (uses nextest for speed)
test-coverage:
    @echo "🧪 Running tests with coverage (nextest + llvm-cov)..."
    cargo llvm-cov nextest --all-features --workspace --lcov --output-path coverage/lcov.info
    cargo llvm-cov report --html --output-dir coverage/html
    @echo "📊 Coverage reports generated:"
    @echo "  - LCOV: coverage/lcov.info"
    @echo "  - HTML: coverage/html/index.html"

# Run tests in watch mode
test-watch:
    @echo "🧪 Running tests in watch mode..."
    cargo watch -x test

# Download CCL test data from ccl-test-data repository
download-ccl-tests:
    @echo "📥 Downloading CCL test data from ccl-test-data repository..."
    @echo "Cloning repository to temporary location..."
    @rm -rf /tmp/ccl-test-data
    @git clone --depth 1 --quiet https://github.com/tylerbutler/ccl-test-data.git /tmp/ccl-test-data
    @mkdir -p crates/sickle/tests/test_data
    @echo "Copying test files..."
    @cp /tmp/ccl-test-data/generated_tests/*.json crates/sickle/tests/test_data/
    @rm -rf /tmp/ccl-test-data
    @echo "✅ Downloaded all test files to crates/sickle/tests/test_data/"
    @echo "   Files: $$(ls crates/sickle/tests/test_data/*.json | wc -l) JSON test suites"

# Run CCL test suites with detailed results from all JSON test files
test-ccl:
    @cargo test -p sickle test_all_ccl_suites_comprehensive -- --nocapture

# Generate sickle capabilities documentation from test data
sickle-capabilities:
    @python3 crates/sickle/scripts/generate_capabilities.py

# Benchmarking Commands
# ====================

# Run all benchmarks
bench:
    @echo "🚀 Running benchmarks..."
    cargo bench

# Run specific benchmark with detailed output
bench-subprocess:
    @echo "🚀 Running subprocess performance benchmarks..."
    cargo bench --bench subprocess_performance

# Run benchmarks and save baseline for comparison
bench-baseline name="main":
    @echo "🚀 Running benchmarks and saving baseline: {{name}}"
    cargo bench -- --save-baseline {{name}}

# Compare benchmarks against saved baseline  
bench-compare baseline="main":
    @echo "🚀 Comparing benchmarks against baseline: {{baseline}}"
    cargo bench -- --baseline {{baseline}}

# Code Quality Commands
# ====================

# Run linting with clippy
lint *ARGS='':
    @echo "🔍 Running clippy..."
    cargo clippy {{ARGS}} -- -A clippy::needless_return -D warnings

# Format code
format *ARGS='':
    @echo "🎨 Formatting code..."
    cargo fmt --all -- {{ARGS}}

# Run all linting and formatting checks
check-style:
    @echo "🔍 Running code quality checks..."
    cargo fmt -- --check
    cargo clippy -- -A clippy::needless_return -D warnings
    cargo check

# Auto-fix formatting and simple lint issues
fix:
    @echo "🔧 Auto-fixing code issues..."
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fix --allow-dirty --allow-staged

# Check for unused dependencies (requires nightly)
deps:
    @echo "🔍 Checking for unused dependencies..."
    cargo +nightly udeps

# Security audit
audit:
    @echo "🔒 Running security audit..."
    cargo audit
    cargo deny check

# Check for supply chain vulnerabilities
supply-chain:
    @echo "🔗 Checking supply chain security..."
    cargo deny check bans licenses sources

# Documentation Commands
# ======================

# Generate and open documentation
docs:
    @echo "📚 Generating documentation..."
    cargo doc --open --no-deps

# Generate documentation for all dependencies
docs-full:
    @echo "📚 Generating full documentation..."
    cargo doc --open

# Check documentation for errors
docs-check:
    @echo "📚 Checking documentation..."
    cargo doc --no-deps

# Release Commands
# ===============

# Standard release build
release:
    @echo "🚀 Building release..."
    cargo build --release

# Verify that packages can be built for crates.io publishing
verify-package:
    @echo "📦 Verifying crate packaging..."
    cargo package --workspace --no-verify --quiet
    @echo "✅ Package verification complete!"

# Perform pre-release checks
pre-release:
    @echo "🚀 Running pre-release checks..."
    just check-style
    just test
    just audit
    just build-release
    just verify-package
    just dist-plan
    @echo "✅ Pre-release checks complete!"

# Build release binaries for all platforms
release-build:
    @echo "🚀 Building release binaries..."
    cargo build --release --target x86_64-unknown-linux-gnu
    cargo build --release --target aarch64-unknown-linux-gnu
    cargo build --release --target x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-pc-windows-gnu

# Package release artifacts
package:
    @echo "📦 Packaging release artifacts..."
    mkdir -p dist/
    cp target/release/santa dist/santa-linux-x64 2>/dev/null || echo "Linux x64 binary not found"
    cp target/release/santa dist/santa-linux-arm64 2>/dev/null || echo "Linux ARM64 binary not found"
    [ -f dist/santa-linux-x64 ] && tar -czf dist/santa-linux-x64.tar.gz -C dist santa-linux-x64
    [ -f dist/santa-linux-arm64 ] && tar -czf dist/santa-linux-arm64.tar.gz -C dist santa-linux-arm64

# cargo-dist Commands
# ===================

# Test local cargo-dist build
dist-build:
    @echo "🚀 Building with cargo-dist..."
    ~/.cargo/bin/dist build

# Preview what cargo-dist will release
dist-plan:
    @echo "📋 Planning release with cargo-dist..."
    ~/.cargo/bin/dist plan

# Re-run cargo-dist initialization
dist-init:
    @echo "🔧 Running cargo-dist init..."
    ~/.cargo/bin/dist init

# Development Workflow Commands
# ============================

# Full development check - run before committing
check-all:
    @echo "🔍 Running complete development checks..."
    just check-style
    just test
    just deps
    just audit
    @echo "✅ All checks passed!"

# Quick development check for faster iteration
check-quick:
    @echo "⚡ Running quick checks..."
    cargo check
    cargo test --lib
    @echo "✅ Quick checks passed!"

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean
    rm -rf coverage/
    rm -rf dist/

# Install santa locally for testing
install-local:
    @echo "📦 Installing santa locally..."
    cargo install --path . --force

# Utility Commands
# ===============

# Show project statistics
stats:
    @echo "📊 Project Statistics:"
    @echo "Lines of code:"
    @find src -name "*.rs" -exec wc -l {} + | tail -1
    @echo ""
    @echo "Dependencies:"
    @cargo tree --depth 1

# Show current version
version:
    @cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version' 2>/dev/null || grep '^version' Cargo.toml | head -1

# Show help for shell completions
completions:
    @echo "🐚 Shell Completions Setup:"
    @echo ""
    @echo "Bash:"
    @echo "  santa completions bash >> ~/.bashrc"
    @echo "  # or system-wide:"
    @echo "  santa completions bash | sudo tee /etc/bash_completion.d/santa"
    @echo ""
    @echo "Zsh:"
    @echo "  santa completions zsh >> ~/.zshrc"  
    @echo "  # or to completions directory:"
    @echo "  santa completions zsh > ~/.local/share/zsh/site-functions/_santa"
    @echo ""
    @echo "Fish:"
    @echo "  santa completions fish >> ~/.config/fish/config.fish"
    @echo "  # or to completions directory:"
    @echo "  santa completions fish > ~/.config/fish/completions/santa.fish"

# Environment variable help
env-help:
    @echo "🌍 Environment Variables:"
    @echo ""
    @echo "Configuration:"
    @echo "  SANTA_LOG_LEVEL         Set log level (trace, debug, info, warn, error)"
    @echo "  SANTA_CONFIG_PATH       Override path to configuration file"
    @echo "  SANTA_SOURCES           Override package sources (comma-separated: brew,cargo,apt)"
    @echo "  SANTA_PACKAGES          Override package list (comma-separated)"
    @echo "  SANTA_BUILTIN_ONLY      Use builtin configuration only (true/false)"
    @echo "  SANTA_VERBOSE           Set verbose logging level (0-3)"
    @echo "  SANTA_DATA_DIR          Override data directory path"
    @echo ""
    @echo "Performance:"
    @echo "  SANTA_CACHE_TTL_SECONDS Set package cache TTL in seconds"
    @echo "  SANTA_CACHE_SIZE        Set maximum cache size (number of entries)"
    @echo ""
    @echo "Advanced:"
    @echo "  SANTA_HOT_RELOAD        Enable configuration hot-reloading (true/false)"

# Development server with hot reload
dev:
    @echo "🔄 Starting development server with hot reload..."
    cargo watch -x 'run -- --help'

# Demo the application with example usage
demo:
    @echo "🎬 Santa Demo:"
    @echo ""
    @echo "1. Show help:"
    cargo run -- --help
    @echo ""
    @echo "2. Show status (builtin mode):"
    cargo run -- --builtin-only status
    @echo ""
    @echo "3. Show config (builtin mode):"
    cargo run -- --builtin-only config

# CI/CD Commands (matches GitHub Actions)
# =====================================

# Run the same checks as CI
ci:
    @echo "🤖 Running CI checks locally..."
    just check-style
    just test
    just build-release
    just audit
    @echo "✅ CI checks complete!"

# Platform-specific CI simulation
ci-linux:
    @echo "🐧 Running Linux CI simulation..."
    cargo test --target x86_64-unknown-linux-gnu
    cargo build --release --target x86_64-unknown-linux-gnu

ci-macos:
    @echo "🍎 Running macOS CI simulation..."
    cargo test --target x86_64-apple-darwin
    cargo build --release --target x86_64-apple-darwin

ci-windows:
    @echo "🪟 Running Windows CI simulation..."
    cargo test --target x86_64-pc-windows-gnu
    cargo build --release --target x86_64-pc-windows-gnu

# Maintenance Commands
# ===================

# Update all dependencies
update-deps:
    @echo "📦 Updating dependencies..."
    cargo update
    @echo "✅ Dependencies updated! Run 'just test' to verify."

# Check for outdated dependencies
check-outdated:
    @echo "📦 Checking for outdated dependencies..."
    cargo outdated || echo "Install cargo-outdated with: cargo install cargo-outdated"

# Generate security advisory report
security-report:
    @echo "🔒 Generating security report..."
    cargo audit --output json > security-report.json
    @echo "📄 Security report saved to security-report.json"

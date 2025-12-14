#!/usr/bin/env just --justfile

# Santa Package Manager - Development Commands
#
# This justfile provides convenient commands for development, testing, and deployment.
# Install just: https://github.com/casey/just#installation

# ===================
# Aliases
# ===================

alias b := build
alias br := build-release
alias t := test
alias ta := test-all
alias tf := test-fast
alias l := lint
alias f := fix
alias pr := ci

export RUST_BACKTRACE := "1"

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default recipe - shows available commands
default:
    @just --list

# ===================
# Development Setup
# ===================

# Install all development dependencies
_install-udeps:
    cargo install cargo-udeps --locked

_install-nextest:
    cargo install cargo-nextest --locked

_install-llvm-cov:
    cargo install cargo-llvm-cov --locked

_install-audit:
    cargo install cargo-audit --locked

_install-deny:
    cargo install cargo-deny --locked

_install-watch:
    cargo install cargo-watch --locked

_install-outdated:
    cargo install cargo-outdated --locked

_install-dist:
    cargo install cargo-dist --locked

# Build the project in debug mode
build *ARGS='':
    cargo build {{ARGS}}

# Build the project in release mode
build-release *ARGS='':
    cargo build --release {{ARGS}}

# Generate package index from source files
generate-index:
    @echo "📋 Generating package index from source files..."
    cargo run --bin generate-index
    @echo "✅ Package index generated at crates/santa-cli/data/known_packages.ccl"

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
# ===================

# Run all tests with cargo test
test *ARGS='':
    cargo test {{ARGS}}

# Run tests with nextest (faster parallel execution)
test-fast *ARGS='':
    cargo nextest run {{ARGS}}

# Run tests with all features enabled
test-all *ARGS='':
    cargo test --all-features {{ARGS}}

# Run tests in watch mode
test-watch:
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
    cargo bench

# Code Quality Commands
# ====================

# Run linting with clippy
lint *ARGS='':
    cargo clippy {{ARGS}} -- -A clippy::needless_return -D warnings

# Format code
format *ARGS='':
    cargo fmt --all -- {{ARGS}}

# Auto-fix formatting and simple lint issues
fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fix --allow-dirty --allow-staged

# Quick development check for faster iteration
check-quick:
    @echo "⚡ Running quick checks..."
    cargo check
    cargo test --lib
    @echo "✅ Quick checks passed!"

# Security audit
audit:
    cargo audit
    cargo deny check

# Check for semver-incompatible changes
semver:
    cargo semver-checks

# Documentation Commands
# ======================

# Generate CLI help in markdown format
markdown-help:
    @echo "📖 Generating CLI markdown help..."
    @mkdir -p docs
    cargo run -p santa --quiet -- --markdown-help > docs/cli-reference.md
    @echo "✅ Generated docs/cli-reference.md"

# Generate and open documentation
docs:
    cargo doc --open --no-deps

# Release Commands
# ===============

# Standard release build
release:
    cargo build --release

# Verify packages can be packaged (validates metadata and structure)
# Uses --no-verify because path deps may not be published to crates.io yet
verify-package:
    cargo package --no-verify -p sickle
    cargo package --no-verify -p santa-data
    cargo package --no-verify -p santa

# Development Workflow Commands
# ============================

# Clean build artifacts
clean:
    cargo clean

# CI/CD Commands (matches GitHub Actions)
# =====================================

# Run the same checks as CI
ci:
    cargo fmt -- --check
    cargo clippy -- -A clippy::needless_return -D warnings
    cargo test
    cargo build --release
    cargo audit

# ===================
# Binary Size Analysis (Linux only)
# ===================

# Run cargo-bloated on santa and sickle, save to metrics/ (Linux only)
[linux]
bloat:
    cargo bloated -p santa --output crates | tee metrics/bloat.txt
    cargo bloated --lib -p sickle --all-features --output crates | tee metrics/bloat-sickle.txt

# Record release binary size to metrics/binary-size.txt
[linux]
record-size:
    #!/usr/bin/env bash
    set -euo pipefail
    BINARY="target/release/santa"
    if [ ! -f "$BINARY" ]; then
        echo "Release binary not found. Run 'just build-release' first."
        exit 1
    fi
    VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "santa") | .version')
    SIZE=$(stat --format="%s" "$BINARY")
    HUMAN=$(numfmt --to=iec --suffix=B "$SIZE")
    DATE=$(date +%Y-%m-%d)
    echo "$DATE v$VERSION $SIZE $HUMAN" >> metrics/binary-size.txt
    echo "Recorded: $DATE v$VERSION $SIZE ($HUMAN)"

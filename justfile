#!/usr/bin/env just --justfile

# Santa Package Manager - Development Commands
#
# This justfile provides convenient commands for development, testing, and deployment.
# Install just: https://github.com/casey/just#installation

export RUST_BACKTRACE := "1"

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

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
alias c := check-quick
alias pr := ci

# Default recipe - shows available commands
default:
    @just --list

# ===================
# Development Setup
# ===================

# Install all development dependencies via mise
setup:
    @echo "🔧 Setting up development environment..."
    @command -v mise >/dev/null 2>&1 || { echo "❌ mise not found. Install from https://mise.jdx.dev"; exit 1; }
    mise install
    @echo "✅ Development setup complete!"
    @echo ""
    @echo "Installed tools:"
    @mise list --current

# ===================
# Build Commands
# ===================

# Build the project in debug mode
build *ARGS='':
    @echo "🔨 Building santa (debug)..."
    cargo build {{ARGS}}
    @just markdown-help

# Build the project in release mode
build-release *ARGS='':
    @echo "🔨 Building santa (release)..."
    cargo build --release {{ARGS}}

# ===================
# Testing Commands
# ===================

# Run all tests with cargo test
test *ARGS='':
    @echo "🧪 Running tests..."
    cargo test {{ARGS}}

# Run tests with nextest (faster parallel execution)
test-fast *ARGS='':
    @echo "🧪 Running tests with nextest..."
    cargo nextest run {{ARGS}}

# Run tests with all features enabled
test-all *ARGS='':
    @echo "🧪 Running tests with all features..."
    cargo test --all-features {{ARGS}}

# Run tests in watch mode
test-watch:
    @echo "🧪 Running tests in watch mode..."
    cargo watch -x test

# Run tests with coverage reporting (uses nextest for speed)
test-coverage:
    @echo "🧪 Running tests with coverage (nextest + llvm-cov)..."
    cargo llvm-cov nextest --all-features --workspace --lcov --output-path coverage/lcov.info
    cargo llvm-cov report --html --output-dir coverage/html
    @echo "📊 Coverage reports generated:"
    @echo "  - LCOV: coverage/lcov.info"
    @echo "  - HTML: coverage/html/index.html"

# Run sickle data-driven tests with coverage
test-coverage-sickle:
    @echo "🧪 Running sickle data-driven tests with coverage..."
    cargo llvm-cov nextest -p sickle --all-features \
      --lcov --output-path coverage/sickle-lcov.info \
      -E 'binary(data_driven_tests)'
    cargo llvm-cov report --html --output-dir coverage/sickle-html
    @echo "📊 Sickle coverage: coverage/sickle-html/index.html"

# ===================
# CCL/Sickle Commands
# ===================

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

# ===================
# Code Quality
# ===================

# Run linting with clippy
lint *ARGS='':
    @echo "🔍 Running clippy..."
    cargo clippy {{ARGS}} -- -A clippy::needless_return -D warnings

# Format code
format *ARGS='':
    @echo "🎨 Formatting code..."
    cargo fmt --all -- {{ARGS}}

# Auto-fix formatting and simple lint issues
fix:
    @echo "🔧 Auto-fixing code issues..."
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fix --allow-dirty --allow-staged

# Quick development check for faster iteration
check-quick:
    @echo "⚡ Running quick checks..."
    cargo check
    cargo test --lib
    @echo "✅ Quick checks passed!"

# Check for unused dependencies (requires nightly)
unused-deps:
    @echo "🔍 Checking for unused dependencies..."
    cargo +nightly udeps

# Security audit
audit:
    @echo "🔒 Running security audit..."
    cargo audit
    cargo deny check

# Check for semver-incompatible changes
semver:
    @echo "🔍 Checking semver compatibility..."
    cargo semver-checks

# ===================
# Documentation
# ===================

# Generate CLI help in markdown format
markdown-help:
    @echo "📖 Generating CLI markdown help..."
    @mkdir -p docs
    cargo run -p santa --quiet -- --markdown-help > docs/cli-reference.md
    @echo "✅ Generated docs/cli-reference.md"

# Generate and open documentation
docs:
    @echo "📚 Generating documentation..."
    cargo doc --open --no-deps

# Check documentation for errors (used in CI)
docs-check:
    @echo "📚 Checking documentation..."
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --workspace

# ===================
# Benchmarking
# ===================

# Run all benchmarks
bench:
    @echo "🚀 Running benchmarks..."
    cargo bench

# Run benchmarks and save baseline
bench-baseline:
    @echo "🚀 Running benchmarks and saving baseline: main"
    cargo bench -- --save-baseline main

# Compare benchmarks against saved baseline
bench-compare:
    @echo "🚀 Comparing benchmarks against baseline: main"
    cargo bench -- --baseline main

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

# ===================
# Utilities
# ===================

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

# Verify that packages can be built for crates.io publishing
verify-package:
    @echo "📦 Verifying crate packaging..."
    cargo package --workspace --no-verify --quiet
    @echo "✅ Package verification complete!"

# Run comprehensive CI checks locally (matches PR workflow)
ci:
    @echo "🤖 Running CI checks locally..."
    just format --check
    just lint
    just test
    just audit
    just build-release
    just verify-package
    @echo "✅ CI checks complete!"

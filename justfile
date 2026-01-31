#!/usr/bin/env just --justfile

# Santa Package Manager - Development Commands
#
# This justfile provides convenient commands for development, testing, and deployment.
# Install just: https://github.com/casey/just#installation

# Common aliases for faster development
alias b := build
alias br := build-release
alias r := release
alias t := test
alias ta := test-all
alias tf := test-fast
alias l := lint
alias f := fix
alias ci := pr

export RUST_BACKTRACE := "1"

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default recipe - shows available commands
default:
    @just --list

# Development Commands
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

# Generate package index from source files (verified packages only)
generate-index *ARGS='':
    @cargo run --quiet --features dev-tools --bin generate-index -- {{ARGS}}

# Migrate packages from known_packages.ccl to source files
migrate-sources:
    @echo "📋 Migrating packages to source files..."
    @cargo run --quiet --features dev-tools --bin migrate-sources
    @echo "✅ Packages migrated to crates/santa-cli/data/sources/"

# Merge verified packages into source files
merge-verified:
    @echo "📋 Merging verified packages into source files..."
    @cargo run --quiet --features dev-tools --bin merge-verified
    @echo "✅ Verified packages merged"

# Collect packages from all sources
collect-packages *ARGS='':
    @echo "📦 Collecting packages from sources..."
    @cargo run --quiet --features dev-tools --bin collect-packages -- {{ARGS}}

# Cross-reference and score packages
crossref-packages *ARGS='':
    @echo "🔗 Cross-referencing packages..."
    @cargo run --quiet --features dev-tools --bin crossref-packages -- {{ARGS}}

# Verify package availability
verify-packages *ARGS='':
    @echo "✓ Verifying packages..."
    @cargo run --quiet --features dev-tools --bin verify-packages -- {{ARGS}}

# Fetch package name mappings from Repology
fetch-repology *ARGS='':
    @cargo run --quiet --features dev-tools --bin fetch-repology -- {{ARGS}}

# Full package discovery pipeline
pipeline:
    @echo "Running full package discovery pipeline..."
    just collect-packages
    just crossref-packages --top=500
    just verify-packages
    just build-repology-cache --from-crossref 200
    just validate-cached
    just merge-verified
    just generate-index
    @echo "Pipeline complete"

# Query Repology for top packages from crossref and update source files
fetch-repology-from-crossref limit='100':
    @cargo run --quiet --features dev-tools --bin fetch-repology -- query --from-crossref {{limit}} --update

# Build Repology cache from crossref or source files
build-repology-cache *ARGS='':
    @cargo run --quiet --features dev-tools --bin fetch-repology -- build-cache {{ARGS}}

# Validate source CCL files using cached Repology data (fast)
validate-cached *ARGS='all':
    @cargo run --quiet --features dev-tools --bin fetch-repology -- validate {{ARGS}} --from-cache

# Validate source CCL files against Repology live API (slow)
validate-sources *SOURCES='all':
    @cargo run --quiet --features dev-tools --bin fetch-repology -- validate {{SOURCES}}

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
    cargo test {{ARGS}}

# Run tests with nextest (faster parallel execution)
test-fast *ARGS='':
    cargo nextest run {{ARGS}}

# Run only unit tests
test-unit:
    cargo test --lib

# Run only integration tests
test-integration:
    cargo test --test '*'

# Run tests with all features enabled (excluding reference_compliant)
test-all *ARGS='':
    @echo "🧪 Running tests with all features..."
    cargo test --features full,unstable {{ARGS}}

# Run tests in watch mode
test-watch:
    cargo watch -x test

# Run tests with coverage reporting (uses nextest for speed)
test-coverage:
    @echo "🧪 Running tests with coverage (nextest + llvm-cov)..."
    cargo llvm-cov nextest --features full,unstable --workspace --lcov --output-path coverage/lcov.info
    cargo llvm-cov report --html --output-dir coverage/html
    @echo "📊 Coverage reports generated:"
    @echo "  - LCOV: coverage/lcov.info"
    @echo "  - HTML: coverage/html/index.html"

# Run sickle data-driven tests with coverage
test-coverage-sickle:
    @echo "🧪 Running sickle data-driven tests with coverage..."
    cargo llvm-cov nextest -p sickle --features full,unstable \
      --lcov --output-path coverage/sickle-lcov.info \
      -E 'binary(data_driven_tests)'
    cargo llvm-cov report --html --output-dir coverage/sickle-html
    @echo "📊 Sickle coverage: coverage/sickle-html/index.html"

# Generate HTML coverage report (run after test-coverage)
coverage-report:
    cargo llvm-cov report --html --output-dir target/llvm-cov/html

# Download CCL test data from latest ccl-test-data release
download-ccl-tests:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📥 Downloading CCL test data from latest release..."
    echo "Fetching latest release information..."
    mkdir -p crates/sickle/tests/test_data
    VERSION=$(curl -sL https://api.github.com/repos/CatConfLang/ccl-test-data/releases/latest | jq -r '.tag_name')
    echo "Downloading generated test data zip from $VERSION release..."
    
    # Get the zip file URL for generated tests
    ZIP_URL=$(curl -sL https://api.github.com/repos/CatConfLang/ccl-test-data/releases/latest | \
    jq -r '.assets[] | select(.name | contains("generated")) | .browser_download_url')
    
    # Create temporary directory for extraction
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # Download and extract the zip file
    echo "Downloading $ZIP_URL..."
    curl -sL "$ZIP_URL" -o "$TEMP_DIR/generated-tests.zip"
    echo "Extracting test files..."
    unzip -q "$TEMP_DIR/generated-tests.zip" -d "$TEMP_DIR/"
    
    # Copy JSON files to test data directory
    echo "Copying test files to crates/sickle/tests/test_data/"
    find "$TEMP_DIR" -name "*.json" -exec cp {} crates/sickle/tests/test_data/ \;
    
    echo "✅ Downloaded and extracted all test files to crates/sickle/tests/test_data/"

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
    ruff format scripts/

# Auto-fix formatting and simple lint issues
fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged
    cargo fix --allow-dirty --allow-staged

# Check for unused dependencies (requires nightly)
deps:
    cargo +nightly udeps

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

# Check documentation builds without warnings (for CI)
docs-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --workspace

# Changelog Commands
# ==================

# Regenerate all configs from commit-types.json (single source of truth)
generate-configs:
    python3 scripts/generate-cliff-configs.py
    python3 scripts/generate-commitlint-config.py

# Check that generated configs are in sync with commit-types.json
check-configs:
    python3 scripts/check-configs-sync.py

# Regenerate git-cliff config files for all crates
generate-cliff-configs:
    python3 scripts/generate-cliff-configs.py

# Generate changelogs for all crates
changelogs: generate-cliff-configs
    git-cliff --config crates/sickle/cliff.toml -o crates/sickle/CHANGELOG.md 2>/dev/null
    git-cliff --config crates/santa-data/cliff.toml -o crates/santa-data/CHANGELOG.md 2>/dev/null
    git-cliff --config crates/santa-cli/cliff.toml -o crates/santa-cli/CHANGELOG.md 2>/dev/null

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

# Run PR checks (mimics pr.yml workflow)
pr:
    just format --check
    just docs-check
    just lint
    just check-configs
    just test-coverage
    just audit
    just build
    just verify-package

# Run main branch checks (mimics test.yml workflow)
main:
    just format --check
    just docs-check
    just lint
    just test-coverage
    just audit
    just build-release

# Binary Size Analysis Commands
# =============================

# Run cargo-bloated on santa and sickle, save to metrics/ (Linux only)
# Note: sickle is built as dylib temporarily for analysis (not in Cargo.toml to support panic=abort downstream)
[linux]
bloat:
    cargo bloated -p santa --bin=santa --output crates | tee metrics/bloat.txt
    cargo rustc -p sickle --lib --all-features --release --crate-type=dylib
    cargo bloated -p sickle --lib --all-features --output crates | tee metrics/bloat-sickle.txt

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

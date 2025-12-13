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
alias pr := ci

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

# Run tests with all features enabled
test-all *ARGS='':
    cargo test --all-features {{ARGS}}

# Run tests in watch mode
test-watch:
    cargo watch -x test

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

# Check for unused dependencies (requires nightly)
deps:
    cargo +nightly udeps

# Security audit
audit:
    cargo audit
    cargo deny check

# Documentation Commands
# ======================

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

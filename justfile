#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

alias b := build
alias r := release
alias t := test

build *ARGS='':
  cargo build {{ARGS}}

ci-build TARGET='x86_64-unknown-linux-gnu' *ARGS='':
  cargo build --locked --release --target {{TARGET}} {{ARGS}}

release:
  cargo build --release

test:
  cargo test

lint *ARGS='':
  cargo clippy {{ARGS}} -- -A clippy::needless_return

format *ARGS='':
  cargo fmt --all -- {{ARGS}}

deps:
  cargo +nightly udeps

cbuild target='x86_64-unknown-linux-gnu' *ARGS='':
  cross build --locked --release --target {{target}} {{ARGS}}

#!/usr/bin/env just --justfile

export RUST_BACKTRACE := "1"

alias b := build
alias r := release
alias t := test

build:
  cargo build

release:
  cargo build --release

test:
  cargo test

lint +args:
  cargo clippy {{args}} -- -A clippy::needless_return

deps:
  cargo +nightly udeps

cbuild target='x86_64-unknown-linux-gnu':
  cross build --verbose --locked --release --target {{target}}

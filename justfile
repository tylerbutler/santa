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

cbuild target='x86_64-unknown-linux-gnu':
  cross build --verbose --locked --release --target {{target}}

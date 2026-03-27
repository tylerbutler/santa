# Copilot instructions for Santa

## Build, test, and lint commands

- Treat `justfile` as the canonical source for local commands. Some top-level docs still mention recipes like `just setup`, `just check-quick`, and `just dev`, but those recipes are not present in the current `justfile`.
- Workspace packages:
  - `santa` in `crates/santa-cli`
  - `santa-data` in `crates/santa-data`
  - `sickle` in `crates/sickle`
  - `sickle-cli` in `crates/sickle-cli`
- Common commands:
  - `just build`
  - `just build-release`
  - `just test`
  - `just test-fast`
  - `just test-all`
  - `just lint`
  - `just format --check`
  - `just docs-check`
  - `just audit`
  - `just pr` to mirror the PR workflow locally
- Crate-scoped commands:
  - `cargo build -p santa`
  - `cargo test -p santa`
  - `cargo test -p santa-data`
  - `cargo test -p sickle`
  - `cargo test -p sickle-cli`
- Single-test patterns:
  - `cargo test -p santa test_cli_help`
  - `cargo test -p santa --test integration_tests test_cli_help`
  - `cargo test -p sickle --test integration_tests test_complete_config_file`
  - `just test -p santa --test integration_tests test_cli_help`
- Package data and config maintenance:
  - `just generate-index`
  - `just validate-cached`
  - `just pipeline`
  - `just generate-configs`
  - `just check-configs`

## High-level architecture

- This is a Cargo workspace with four crates. `sickle` is the core CCL parser/model/Serde library, `santa-data` builds typed Santa config and package models on top of it, `santa` is the main CLI/runtime, and `sickle-cli` is a non-published helper CLI for working with CCL.
- The main runtime path in `santa` is: CLI parsing in `crates/santa-cli/src/main.rs` -> config loading and validation in `configuration.rs` plus `configuration/env.rs` -> package/source data assembly in `data.rs` -> layered source/package merging in `data_layers.rs` -> command execution in `commands.rs` using `PackageSource` implementations plus `PackageCache`.
- Package installation is safe by default. `crates/santa-cli/src/script_generator.rs` renders MiniJinja templates from `crates/santa-cli/templates/*.tera` and generates reviewable scripts; direct execution only happens when the user explicitly opts into `-x` / `--execute`.
- Santa’s data model is CCL-first. Manually curated package-manager definitions live in `crates/santa-cli/data/sources/*.ccl`, package catalog metadata lives in `crates/santa-cli/data/packages.ccl`, and `crates/santa-cli/data/known_packages.ccl` is the generated runtime index consumed by the CLI.
- There are two distinct layering flows:
  - Runtime data precedence in `DataLayerManager`: bundled -> downloaded -> user custom, with higher layers replacing lower ones by name.
  - Package curation pipeline documented in `DEVELOPMENT.md`: collect -> cross-reference -> Repology cache/validation -> merge verified entries -> generate the runtime index.
- Tests are crate-local. The `santa` crate combines `assert_cmd` integration tests with property and security tests under `crates/santa-cli/tests`, while `sickle` has feature-gated integration and data-driven parser tests under `crates/sickle/tests`.

## Key conventions

- Use Cargo package names, not directory names, in commands. The main CLI crate lives in `crates/santa-cli`, but the package name is `santa`, so commands should use `-p santa`, not `-p santa-cli`.
- Treat `crates/santa-cli/data/sources/*.ccl` and `crates/santa-cli/data/packages.ccl` as source-of-truth inputs. Do not hand-edit `crates/santa-cli/data/known_packages.ccl`; regenerate it with `just generate-index`.
- Changes to install/check behavior should preserve the safe script-generation path. Update the MiniJinja templates and keep escaping/validation inside `ScriptGenerator`; do not bypass the `ExecutionMode` split with raw shell execution.
- New layering work should go through `crates/santa-cli/src/data_layers.rs`. `source_layers` is only retained for backward-compatible re-exports and should not be the default place for new logic.
- Configuration changes should preserve `SANTA_*` environment overrides and validation through `SantaConfigExt`. Hot reload behavior lives in `crates/santa-cli/src/configuration/watcher.rs`.
- `commit-types.json` is the source of truth for generated commit/cliff config. If commit types change, run `just generate-configs` and verify with `just check-configs`.
- When top-level docs disagree with the code, prefer `justfile`, crate `Cargo.toml` files, and `.github/workflows/*.yml`; several repository docs currently lag behind the actual workspace and recipe list.

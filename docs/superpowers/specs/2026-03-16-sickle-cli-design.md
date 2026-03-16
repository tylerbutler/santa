# Sickle CLI Design

## Overview

A developer tool for working with CCL files: converting between formats, validating, and formatting. Lives in the santa workspace as a new unpublished binary crate.

## Crate Structure

**New crate:** `crates/sickle-cli/` with `publish = false`.

```
crates/sickle-cli/
├── Cargo.toml
└── src/
    ├── main.rs
    └── commands/
        ├── mod.rs
        ├── convert.rs
        ├── validate.rs
        ├── fmt.rs
        ├── view.rs
        └── parse.rs
```

**Binary name:** `sickle`

**Dependencies:**
- `sickle` (path = "../sickle", features = ["full"])
- `clap` (derive, color, wrap_help)
- `serde_json`
- `toml`
- `colored`
- `anyhow`

**Workspace:** Added to `Cargo.toml` workspace members list.

## CLI Interface

```
sickle <command> [file]
```

All commands accept a file path or read from stdin when file is omitted or `-`.

### Priority Commands

#### `sickle convert <file> --from <fmt> --to <fmt>`

Converts between CCL, JSON, and TOML.

- **Formats:** `ccl`, `json`, `toml`
- **`--from`:** Auto-detected from file extension (`.ccl`, `.json`, `.toml`). Required for stdin. Errors on unknown extension.
- **`--to`:** Required.
- **`--pretty` / `--compact`:** Controls JSON output formatting. Default: pretty.
- Output to stdout.

#### `sickle validate <file>`

Parses CCL file and reports errors.

- Exit code 0 on valid, 1 on invalid.
- Error output includes file path, line number, description (e.g., `config.ccl:12: missing '=' delimiter`).
- **`--quiet` / `-q`:** No output on success, just exit code.

#### `sickle fmt <file>`

Reformats CCL to canonical form.

- Prints to stdout by default.
- **`--in-place` / `-i`:** Overwrites the file.
- Uses sickle's `CclPrinter` / `round_trip`.

### Secondary Commands

#### `sickle view <file>`

Pretty-prints CCL with syntax highlighting (colored keys/values). Always to stdout.

#### `sickle parse <file>`

Debug view showing flat `Entry` list (key, value).

- **`--json`:** Output entries as JSON array.

## Input Handling

Shared `InputSource` abstraction:

```rust
enum InputSource {
    File(PathBuf),
    Stdin,
}
```

- File present: read file, infer `--from` from extension.
- File omitted or `-`: read stdin, `--from` required.
- Unknown extension: error suggesting `--from`.

Extension mapping: `.ccl` → ccl, `.json` → json, `.toml` → toml.

## Format Bridging Strategy

The CLI handles format conversion without changes to the sickle library:

- **CCL → JSON:** `sickle::load()` → `CclObject` → `serde_json::to_string()`
- **CCL → TOML:** `sickle::load()` → `CclObject` → `toml::to_string()`
- **JSON → CCL:** `serde_json::from_str()` → `serde_json::Value` → `sickle::to_string()`
- **TOML → CCL:** `toml::from_str()` → `toml::Value` → `sickle::to_string()`
- **JSON ↔ TOML:** Direct via serde_json::Value / toml::Value (bonus, falls out naturally)

No sickle lib changes needed for v1.

## Error Reporting

- Parse errors: `file:line: description`
- Conversion errors: clear message about what failed
- File errors: path included in message
- `anyhow` for error chaining, `colored` for terminal output
- Errors to stderr, data to stdout (unix convention)

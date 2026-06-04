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
- `toml` (version "0.8")
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
- **`--yes` / `-y`:** Skip interactive prompts (e.g., comment loss warning). Useful for scripting.
- Output to stdout. File output via shell redirection (`> file.json`).
- All conversions between all three formats are supported, including JSON ↔ TOML (which doesn't involve CCL at all but falls out naturally).

**Comment loss warning:** When converting FROM CCL to another format, the CLI checks if the input contains CCL comments (`/= ...`). If comments are present, the user is prompted with a warning that comments will be lost in the conversion and must acknowledge before proceeding. Use `--yes` / `-y` to skip the prompt (for scripts/CI).

#### `sickle validate <file>`

Parses CCL file and reports errors.

- Exit code 0 on valid, 1 on invalid.
- Error output includes file path and description (e.g., `config.ccl: missing '=' delimiter`).
- **`--quiet` / `-q`:** No output on success, just exit code.

**Note:** sickle's `Error` type does not currently include line numbers. Error messages will include the file path and error description but not line/column positions. Line number reporting is a future improvement requiring parser changes.

#### `sickle fmt <file>`

Reformats CCL to canonical form using structure-preserving formatting.

- Prints to stdout by default.
- **`--in-place` / `-i`:** Overwrites the file.
- **Pipeline:** `parse()` → entry-level `print()` (preserves document structure, normalizes whitespace). This is intentionally NOT model-level printing, which would lose entry ordering and structure.

### Secondary Commands

#### `sickle view <file>`

Pretty-prints CCL with syntax highlighting (colored keys/values). Always to stdout. Color scheme deferred to implementation.

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

The CLI handles format conversion. The CCL ↔ other-format path requires a `CclObject` → `serde_json::Value` conversion because `CclObject`'s derived `Serialize` produces the internal recursive representation (`{"key": [{"value": [{}]}]}`), not natural JSON (`{"key": "value"}`).

**Conversion pipelines:**

- **CCL → JSON:** `sickle::load()` → `CclObject` → CLI-side `ccl_to_value()` → `serde_json::to_string()`
- **CCL → TOML:** `sickle::load()` → `CclObject` → CLI-side `ccl_to_value()` → `toml::to_string()`
- **JSON → CCL:** `serde_json::from_str()` → `serde_json::Value` → CLI-side `value_to_ccl_string()` (recursive walk producing CCL text)
- **TOML → CCL:** `toml::from_str()` → `toml::Value` → CLI-side `value_to_ccl_string()`
- **JSON ↔ TOML:** Direct via `serde_json::Value` / `toml::Value` serialization.

The `ccl_to_value()` and `value_to_ccl_string()` functions live in the CLI crate (e.g., `src/convert_bridge.rs`). If these prove generally useful, they can be promoted to the sickle library later.

**Known limitations for v1:**
- JSON `null` values have no CCL equivalent — converted to empty string with a warning.
- Mixed-type JSON arrays may not round-trip cleanly through CCL.
- Deeply nested structures may produce verbose CCL output.

**No sickle lib changes needed for v1.**

## Error Reporting

- Parse errors: file path + error description (no line numbers in v1)
- Conversion errors: clear message about what failed (e.g., "Cannot represent null in CCL")
- File errors: path included in message
- `anyhow` for error chaining, `colored` for terminal output
- Errors to stderr, data to stdout (unix convention)

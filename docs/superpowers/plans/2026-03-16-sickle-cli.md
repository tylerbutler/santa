# Sickle CLI Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create an unpublished binary crate (`sickle-cli`) providing a CLI for converting, validating, and formatting CCL files.

**Architecture:** New `crates/sickle-cli/` crate with clap derive-based CLI. Format bridging (CclObject ↔ serde_json::Value) handled by a dedicated `bridge` module. All commands share an `InputSource` abstraction for file/stdin handling.

**Tech Stack:** Rust, clap (derive), serde_json, toml, colored, anyhow, sickle (path dep with `full` feature)

**Spec:** `docs/superpowers/specs/2026-03-16-sickle-cli-design.md`

---

## Chunk 1: Crate scaffold and input handling

### Task 1: Create crate scaffold

**Files:**
- Create: `crates/sickle-cli/Cargo.toml`
- Create: `crates/sickle-cli/src/main.rs`
- Modify: `Cargo.toml` (workspace members list, line 3-7)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "sickle-cli"
version = "0.1.0"
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "CLI tool for working with CCL files"
publish = false

[dependencies]
sickle = { path = "../sickle", features = ["full"] }
clap = { version = "4.5", features = ["color", "derive", "wrap_help"] }
serde_json.workspace = true
serde = { workspace = true }
toml = "0.8"
colored.workspace = true
anyhow.workspace = true
dialoguer.workspace = true

[[bin]]
name = "sickle"
path = "src/main.rs"
```

- [ ] **Step 2: Create minimal main.rs**

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod input;

/// A developer tool for working with CCL files
#[derive(Parser)]
#[clap(version, about, max_term_width = 100)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert between CCL, JSON, and TOML formats
    Convert(commands::convert::ConvertArgs),
    /// Validate a CCL file
    Validate(commands::validate::ValidateArgs),
    /// Format a CCL file to canonical form
    Fmt(commands::fmt::FmtArgs),
    /// Pretty-print a CCL file with syntax highlighting
    View(commands::view::ViewArgs),
    /// Show flat parsed entries (debug view)
    Parse(commands::parse::ParseArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert(args) => commands::convert::run(args),
        Commands::Validate(args) => commands::validate::run(args),
        Commands::Fmt(args) => commands::fmt::run(args),
        Commands::View(args) => commands::view::run(args),
        Commands::Parse(args) => commands::parse::run(args),
    }
}
```

- [ ] **Step 3: Add to workspace members**

In `Cargo.toml` (workspace root), add `"crates/sickle-cli"` to the members array:

```toml
members = [
    "crates/santa-cli",
    "crates/santa-data",
    "crates/sickle",
    "crates/sickle-cli",
]
```

- [ ] **Step 4: Create stub modules so it compiles**

Create `crates/sickle-cli/src/input.rs`:

```rust
use anyhow::{bail, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Supported formats
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Format {
    Ccl,
    Json,
    Toml,
}

/// Where input comes from
pub enum InputSource {
    File(PathBuf),
    Stdin,
}

/// Resolved input: content + detected format
pub struct Input {
    pub content: String,
    pub source_name: String,
}

impl InputSource {
    /// Build from optional file argument
    pub fn from_arg(file: Option<&Path>) -> Self {
        match file {
            Some(p) if p.to_str() != Some("-") => InputSource::File(p.to_path_buf()),
            _ => InputSource::Stdin,
        }
    }

    /// Read the content
    pub fn read(&self) -> Result<Input> {
        match self {
            InputSource::File(path) => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
                Ok(Input {
                    content,
                    source_name: path.display().to_string(),
                })
            }
            InputSource::Stdin => {
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content)?;
                Ok(Input {
                    content,
                    source_name: "<stdin>".to_string(),
                })
            }
        }
    }
}

/// Detect format from file extension, or require explicit --from
pub fn detect_format(file: Option<&Path>, explicit: Option<Format>) -> Result<Format> {
    if let Some(fmt) = explicit {
        return Ok(fmt);
    }
    match file {
        Some(p) if p.to_str() != Some("-") => match p.extension().and_then(|e| e.to_str()) {
            Some("ccl") => Ok(Format::Ccl),
            Some("json") => Ok(Format::Json),
            Some("toml") => Ok(Format::Toml),
            Some(ext) => bail!(
                "Unknown extension '.{}'. Use --from to specify format.",
                ext
            ),
            None => bail!("No file extension. Use --from to specify format."),
        },
        _ => bail!("Reading from stdin requires --from to specify format."),
    }
}
```

Create `crates/sickle-cli/src/commands/mod.rs`:

```rust
pub mod convert;
pub mod fmt;
pub mod parse;
pub mod validate;
pub mod view;
```

Create stub files for each command (`convert.rs`, `validate.rs`, `fmt.rs`, `view.rs`, `parse.rs`) — each with:

```rust
use anyhow::Result;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct <Name>Args {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,
}

pub fn run(_args: <Name>Args) -> Result<()> {
    todo!()
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p sickle-cli`
Expected: compiles with no errors (warnings about todo! are fine)

- [ ] **Step 6: Commit**

```bash
git add crates/sickle-cli/ Cargo.toml Cargo.lock
git commit -m "feat(sickle-cli): scaffold new CLI crate

Unpublished binary crate for working with CCL files.
Includes clap CLI skeleton, input handling, and stub commands."
```

---

### Task 2: Implement the format bridge module

This is the core conversion logic between CclObject and serde_json::Value.

**Files:**
- Create: `crates/sickle-cli/src/bridge.rs`
- Create: `crates/sickle-cli/tests/bridge_tests.rs`

- [ ] **Step 1: Write failing tests for ccl_to_value**

Create `crates/sickle-cli/tests/bridge_tests.rs`:

```rust
use serde_json::json;

// We'll test through the public API once bridge is wired up.
// For now, test the bridge module directly.

#[path = "../src/bridge.rs"]
mod bridge;

#[test]
fn simple_string_value() {
    let obj = sickle::load("name = Alice").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"name": "Alice"}));
}

#[test]
fn multiple_keys() {
    let obj = sickle::load("name = Alice\nage = 30").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"name": "Alice", "age": "30"}));
}

#[test]
fn empty_value() {
    let obj = sickle::load("key =").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"key": ""}));
}

#[test]
fn nested_object() {
    let obj = sickle::load("server =\n  host = localhost\n  port = 8080").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"server": {"host": "localhost", "port": "8080"}}));
}

#[test]
fn bare_list() {
    let obj = sickle::load("items =\n  = apple\n  = banana").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"items": ["apple", "banana"]}));
}

#[test]
fn duplicate_key_list() {
    let obj = sickle::load("tag = web\ntag = api").unwrap();
    let val = bridge::ccl_to_value(&obj);
    assert_eq!(val, json!({"tag": ["web", "api"]}));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sickle-cli --test bridge_tests`
Expected: compilation error (bridge module doesn't exist yet)

- [ ] **Step 3: Implement ccl_to_value**

Create `crates/sickle-cli/src/bridge.rs`:

```rust
use serde_json::Value;
use sickle::CclObject;

/// Convert a CclObject into a natural serde_json::Value.
///
/// Mapping rules:
/// - Empty CclObject → Value::String("") (leaf/terminal)
/// - Single key "" (bare list) → Value::Array of converted children
/// - Multiple values for same key → Value::Array
/// - Single value for key → recursive conversion
/// - Object with named keys → Value::Object
pub fn ccl_to_value(obj: &CclObject) -> Value {
    // Empty object = leaf = empty string
    if obj.is_empty() {
        return Value::String(String::new());
    }

    // Check for bare list: single key that is ""
    let keys: Vec<&String> = obj.keys().collect();
    if keys.len() == 1 && keys[0].is_empty() {
        if let Ok(items) = obj.get_all("") {
            let arr: Vec<Value> = items.iter().map(|child| ccl_to_value(child)).collect();
            return Value::Array(arr);
        }
    }

    // Check for string leaf: single key with one empty child
    // This is CCL's encoding of a string: {"value": [{}]}
    if keys.len() == 1 {
        if let Ok(values) = obj.get_all(keys[0]) {
            if values.len() == 1 && values[0].is_empty() {
                return Value::String(keys[0].clone());
            }
        }
    }

    // Otherwise, build an object
    let mut map = serde_json::Map::new();
    for key in obj.keys() {
        if let Ok(values) = obj.get_all(key) {
            if values.len() == 1 {
                map.insert(key.clone(), ccl_to_value(&values[0]));
            } else {
                // Multiple values = array
                let arr: Vec<Value> = values.iter().map(|v| ccl_to_value(v)).collect();
                map.insert(key.clone(), Value::Array(arr));
            }
        }
    }
    Value::Object(map)
}

/// Convert a serde_json::Value into a CCL string.
///
/// Mapping rules:
/// - Object → key = value pairs (nested objects indented)
/// - Array → bare list syntax (= item)
/// - String → literal value
/// - Number → string representation
/// - Bool → "true" / "false"
/// - Null → empty string (with warning on stderr)
pub fn value_to_ccl_string(value: &Value) -> String {
    value_to_ccl_lines(value, 0).join("\n")
}

fn value_to_ccl_lines(value: &Value, indent: usize) -> Vec<String> {
    let prefix = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            let mut lines = Vec::new();
            for (key, val) in map {
                match val {
                    Value::Object(_) => {
                        lines.push(format!("{}{} =", prefix, key));
                        lines.extend(value_to_ccl_lines(val, indent + 2));
                    }
                    Value::Array(arr) => {
                        lines.push(format!("{}{} =", prefix, key));
                        for item in arr {
                            match item {
                                Value::Object(_) | Value::Array(_) => {
                                    lines.extend(value_to_ccl_lines(item, indent + 2));
                                }
                                _ => {
                                    let s = scalar_to_string(item);
                                    lines.push(format!("{}  = {}", prefix, s));
                                }
                            }
                        }
                    }
                    _ => {
                        let s = scalar_to_string(val);
                        lines.push(format!("{}{} = {}", prefix, key, s));
                    }
                }
            }
            lines
        }
        Value::Array(arr) => {
            let mut lines = Vec::new();
            for item in arr {
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        lines.extend(value_to_ccl_lines(item, indent));
                    }
                    _ => {
                        let s = scalar_to_string(item);
                        lines.push(format!("{}= {}", prefix, s));
                    }
                }
            }
            lines
        }
        _ => {
            vec![format!("{}{}", prefix, scalar_to_string(value))]
        }
    }
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => {
            eprintln!("warning: null value converted to empty string");
            String::new()
        }
        _ => unreachable!("scalar_to_string called with non-scalar"),
    }
}

/// Check if CCL input text contains comments (lines starting with /= or / =)
pub fn has_comments(ccl_text: &str) -> bool {
    ccl_text.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("/=") || trimmed.starts_with("/ =")
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sickle-cli --test bridge_tests`
Expected: all 6 tests pass

- [ ] **Step 5: Add bridge module to main.rs**

Add `mod bridge;` after the existing `mod input;` line in `crates/sickle-cli/src/main.rs`. The bridge tests use `#[path]` to include the file directly, so `mod bridge;` in main.rs is only needed for the command modules (convert) to use `crate::bridge`.

- [ ] **Step 6: Write tests for value_to_ccl_string**

Add to `crates/sickle-cli/tests/bridge_tests.rs`:

```rust
#[test]
fn json_object_to_ccl() {
    let val = json!({"name": "Alice", "age": "30"});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("name = Alice"));
    assert!(ccl.contains("age = 30"));
}

#[test]
fn json_nested_to_ccl() {
    let val = json!({"server": {"host": "localhost", "port": "8080"}});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("server ="));
    assert!(ccl.contains("  host = localhost"));
    assert!(ccl.contains("  port = 8080"));
}

#[test]
fn json_array_to_ccl() {
    let val = json!({"items": ["apple", "banana"]});
    let ccl = bridge::value_to_ccl_string(&val);
    assert!(ccl.contains("items ="));
    assert!(ccl.contains("  = apple"));
    assert!(ccl.contains("  = banana"));
}

#[test]
fn has_comments_detects_comments() {
    assert!(bridge::has_comments("name = Alice\n/= a comment\nage = 30"));
    assert!(bridge::has_comments("  /= indented comment"));
    assert!(!bridge::has_comments("name = Alice\nage = 30"));
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo test -p sickle-cli`
Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/sickle-cli/src/bridge.rs crates/sickle-cli/tests/ crates/sickle-cli/src/main.rs
git commit -m "feat(sickle-cli): add format bridge module

Converts between CclObject and serde_json::Value with natural
JSON output. Handles nested objects, bare lists, duplicate keys,
and null warnings."
```

---

## Chunk 2: Priority commands (convert, validate, fmt)

### Task 3: Implement the convert command

**Files:**
- Modify: `crates/sickle-cli/src/commands/convert.rs`

- [ ] **Step 1: Implement convert command**

Replace the stub in `crates/sickle-cli/src/commands/convert.rs`:

```rust
use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::bridge;
use crate::input::{self, Format, InputSource};

#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Input format (auto-detected from file extension)
    #[clap(long)]
    pub from: Option<Format>,

    /// Output format (required)
    #[clap(long)]
    pub to: Format,

    /// Compact JSON output (default: pretty-printed)
    #[clap(long)]
    pub compact: bool,

    /// Skip interactive prompts (e.g., comment loss warning)
    #[clap(short, long)]
    pub yes: bool,
}

pub fn run(args: ConvertArgs) -> Result<()> {
    let from = input::detect_format(args.file.as_deref(), args.from)?;
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    // Check for comment loss when converting FROM CCL
    if matches!(from, Format::Ccl) && !matches!(args.to, Format::Ccl) {
        if bridge::has_comments(&input.content) && !args.yes {
            warn_comment_loss()?;
        }
    }

    let output = convert(&input.content, from, args.to, !args.compact)?;
    print!("{}", output);
    Ok(())
}

fn warn_comment_loss() -> Result<()> {
    use dialoguer::Confirm;
    let proceed = Confirm::new()
        .with_prompt("Warning: CCL comments will be lost in the conversion. Continue?")
        .default(false)
        .interact()?;
    if !proceed {
        bail!("Conversion cancelled.");
    }
    Ok(())
}

fn convert(content: &str, from: Format, to: Format, pretty: bool) -> Result<String> {
    // Parse input to intermediate serde_json::Value
    let value: serde_json::Value = match from {
        Format::Ccl => {
            let obj = sickle::load(content)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            bridge::ccl_to_value(&obj)
        }
        Format::Json => {
            serde_json::from_str(content)
                .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?
        }
        Format::Toml => {
            let toml_val: toml::Value = toml::from_str(content)
                .map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
            // Convert toml::Value to serde_json::Value via serde
            serde_json::to_value(toml_val)
                .map_err(|e| anyhow::anyhow!("TOML to JSON conversion error: {}", e))?
        }
    };

    // Serialize to target format
    match to {
        Format::Ccl => Ok(bridge::value_to_ccl_string(&value)),
        Format::Json => {
            if pretty {
                serde_json::to_string_pretty(&value)
                    .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))
            } else {
                serde_json::to_string(&value)
                    .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))
            }
        }
        Format::Toml => {
            // serde_json::Value can't directly serialize to toml::Value,
            // so we go through a string round-trip
            let toml_val: toml::Value = serde_json::from_value(value.clone())
                .map_err(|e| anyhow::anyhow!("Cannot represent as TOML: {}", e))?;
            toml::to_string_pretty(&toml_val)
                .map_err(|e| anyhow::anyhow!("TOML serialization error: {}", e))
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p sickle-cli`
Expected: compiles

- [ ] **Step 3: Manual smoke test**

Create a test file `crates/sickle-cli/tests/fixtures/sample.ccl`:

```
name = MyApp
version = 1.0.0
server =
  host = localhost
  port = 8080
```

Run: `cargo run -p sickle-cli -- convert crates/sickle-cli/tests/fixtures/sample.ccl --to json`
Expected: pretty-printed JSON output

Run: `echo '{"name":"test"}' | cargo run -p sickle-cli -- convert --from json --to ccl`
Expected: `name = test`

- [ ] **Step 4: Commit**

```bash
git add crates/sickle-cli/src/commands/convert.rs crates/sickle-cli/tests/fixtures/
git commit -m "feat(sickle-cli): implement convert command

Supports CCL, JSON, and TOML conversions in all directions.
Auto-detects input format from extension, warns about comment
loss when converting from CCL."
```

---

### Task 4: Implement the validate command

**Files:**
- Modify: `crates/sickle-cli/src/commands/validate.rs`

- [ ] **Step 1: Implement validate command**

Replace the stub in `crates/sickle-cli/src/commands/validate.rs`:

```rust
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process;

use crate::input::InputSource;

#[derive(clap::Args)]
pub struct ValidateArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Suppress output on success (exit code only)
    #[clap(short, long)]
    pub quiet: bool,
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    match sickle::load(&input.content) {
        Ok(_) => {
            if !args.quiet {
                eprintln!("{} {}", input.source_name, "OK".green());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", input.source_name, format!("{}", e).red());
            process::exit(1);
        }
    }
}
```

- [ ] **Step 2: Smoke test**

Run: `cargo run -p sickle-cli -- validate crates/sickle-cli/tests/fixtures/sample.ccl`
Expected: `crates/sickle-cli/tests/fixtures/sample.ccl OK` and exit code 0

Run: `echo "bad data" | cargo run -p sickle-cli -- validate`
Expected: error message on stderr, exit code 1

- [ ] **Step 3: Commit**

```bash
git add crates/sickle-cli/src/commands/validate.rs
git commit -m "feat(sickle-cli): implement validate command

Parses CCL and reports errors with file path. Supports --quiet
for exit-code-only mode."
```

---

### Task 5: Implement the fmt command

**Files:**
- Modify: `crates/sickle-cli/src/commands/fmt.rs`

- [ ] **Step 1: Implement fmt command**

Replace the stub in `crates/sickle-cli/src/commands/fmt.rs`:

```rust
use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub struct FmtArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Overwrite the file in place
    #[clap(short = 'i', long = "in-place")]
    pub in_place: bool,
}

pub fn run(args: FmtArgs) -> Result<()> {
    if args.in_place && args.file.is_none() {
        bail!("--in-place requires a file argument (cannot overwrite stdin)");
    }

    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    // Structure-preserving format: parse to entries, print back
    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;
    let formatted = sickle::printer::print(&entries);

    if args.in_place {
        let path = args.file.as_ref().unwrap();
        std::fs::write(path, &formatted)
            .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
    } else {
        print!("{}", formatted);
    }

    Ok(())
}
```

- [ ] **Step 2: Smoke test**

Run: `cargo run -p sickle-cli -- fmt crates/sickle-cli/tests/fixtures/sample.ccl`
Expected: canonical CCL output to stdout

- [ ] **Step 3: Commit**

```bash
git add crates/sickle-cli/src/commands/fmt.rs
git commit -m "feat(sickle-cli): implement fmt command

Structure-preserving formatting using entry-level print.
Supports --in-place for overwriting files."
```

---

## Chunk 3: Secondary commands (view, parse)

### Task 6: Implement the view command

**Files:**
- Modify: `crates/sickle-cli/src/commands/view.rs`

- [ ] **Step 1: Implement view command**

Replace the stub in `crates/sickle-cli/src/commands/view.rs`:

```rust
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub struct ViewArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,
}

pub fn run(args: ViewArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    for entry in &entries {
        if entry.key == "/" {
            // Comment (CCL parses `/= text` as key="/", value="text")
            println!("{}", format!("/= {}", entry.value).dimmed());
        } else if entry.key.is_empty() {
            // Bare list item
            println!("{} {}", "=".dimmed(), entry.value.cyan());
        } else if entry.value.is_empty() {
            // Key with empty value (section header or empty)
            println!("{} {}", entry.key.yellow(), "=".dimmed());
        } else {
            // Normal key = value
            println!(
                "{} {} {}",
                entry.key.yellow(),
                "=".dimmed(),
                entry.value.cyan()
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Smoke test**

Run: `cargo run -p sickle-cli -- view crates/sickle-cli/tests/fixtures/sample.ccl`
Expected: colored output with yellow keys and cyan values

- [ ] **Step 3: Commit**

```bash
git add crates/sickle-cli/src/commands/view.rs
git commit -m "feat(sickle-cli): implement view command

Syntax-highlighted CCL display with colored keys, values,
and comments."
```

---

### Task 7: Implement the parse command

**Files:**
- Modify: `crates/sickle-cli/src/commands/parse.rs`

- [ ] **Step 1: Implement parse command**

Replace the stub in `crates/sickle-cli/src/commands/parse.rs`:

```rust
use anyhow::Result;
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub struct ParseArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Output entries as JSON array
    #[clap(long)]
    pub json: bool,
}

pub fn run(args: ParseArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    if args.json {
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))?;
        println!("{}", json);
    } else {
        for (i, entry) in entries.iter().enumerate() {
            println!(
                "[{}] key={:?} value={:?}",
                i, entry.key, entry.value
            );
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Smoke test**

Run: `cargo run -p sickle-cli -- parse crates/sickle-cli/tests/fixtures/sample.ccl`
Expected: numbered entry list

Run: `cargo run -p sickle-cli -- parse --json crates/sickle-cli/tests/fixtures/sample.ccl`
Expected: JSON array of entries

- [ ] **Step 3: Commit**

```bash
git add crates/sickle-cli/src/commands/parse.rs
git commit -m "feat(sickle-cli): implement parse command

Debug view showing flat entry list with optional JSON output."
```

---

## Chunk 4: Integration tests and final verification

### Task 8: Integration tests

**Files:**
- Create: `crates/sickle-cli/tests/cli_tests.rs`

- [ ] **Step 1: Write CLI integration tests**

Create `crates/sickle-cli/tests/cli_tests.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn sickle() -> Command {
    Command::cargo_bin("sickle").unwrap()
}

#[test]
fn convert_ccl_to_json() {
    sickle()
        .args(["convert", "tests/fixtures/sample.ccl", "--to", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"MyApp\""));
}

#[test]
fn convert_json_to_ccl() {
    sickle()
        .args(["convert", "--from", "json", "--to", "ccl"])
        .write_stdin("{\"name\": \"Alice\"}")
        .assert()
        .success()
        .stdout(predicate::str::contains("name = Alice"));
}

#[test]
fn convert_ccl_to_toml() {
    sickle()
        .args(["convert", "tests/fixtures/sample.ccl", "--to", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name = "));
}

#[test]
fn convert_requires_to_flag() {
    sickle()
        .args(["convert", "tests/fixtures/sample.ccl"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--to"));
}

#[test]
fn convert_stdin_requires_from() {
    sickle()
        .args(["convert", "--to", "json"])
        .write_stdin("name = test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--from"));
}

#[test]
fn validate_valid_file() {
    sickle()
        .args(["validate", "tests/fixtures/sample.ccl"])
        .assert()
        .success();
}

#[test]
fn validate_quiet() {
    sickle()
        .args(["validate", "--quiet", "tests/fixtures/sample.ccl"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn fmt_outputs_canonical() {
    sickle()
        .args(["fmt", "tests/fixtures/sample.ccl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name = MyApp"));
}

#[test]
fn parse_default_output() {
    sickle()
        .args(["parse", "tests/fixtures/sample.ccl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[0]"))
        .stdout(predicate::str::contains("name"));
}

#[test]
fn parse_json_output() {
    sickle()
        .args(["parse", "--json", "tests/fixtures/sample.ccl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"key\""))
        .stdout(predicate::str::contains("\"value\""));
}

#[test]
fn validate_invalid_input() {
    sickle()
        .args(["validate"])
        .write_stdin("this has no equals sign and is just text")
        .assert()
        .failure();
}

#[test]
fn convert_toml_to_json() {
    sickle()
        .args(["convert", "--from", "toml", "--to", "json"])
        .write_stdin("[server]\nhost = \"localhost\"\nport = 8080")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"host\""))
        .stdout(predicate::str::contains("\"localhost\""));
}

#[test]
fn convert_comment_loss_skipped_with_yes() {
    // Create temp file with comments
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("with_comments.ccl");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "/= a comment").unwrap();
    writeln!(f, "name = Alice").unwrap();
    drop(f);

    sickle()
        .args(["convert", path.to_str().unwrap(), "--to", "json", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}
```

- [ ] **Step 2: Add test dependencies to Cargo.toml**

Add to `crates/sickle-cli/Cargo.toml`:

```toml
[dev-dependencies]
assert_cmd = "2.1"
predicates = "3.1"
tempfile = "3.26"
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p sickle-cli`
Expected: all tests pass

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p sickle-cli -- -D warnings`
Expected: no warnings

- [ ] **Step 5: Commit**

```bash
git add crates/sickle-cli/tests/ crates/sickle-cli/Cargo.toml
git commit -m "test(sickle-cli): add integration tests

CLI integration tests covering convert, validate, fmt, parse
commands with file input, stdin, and error cases."
```

---

### Task 9: Final verification

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`
Expected: clean build

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass (existing + new)

- [ ] **Step 3: Verify binary works end-to-end**

Run: `cargo run -p sickle-cli -- --help`
Expected: help text showing all 5 subcommands

Run: `echo '{"database": {"host": "db.example.com", "port": 5432}}' | cargo run -p sickle-cli -- convert --from json --to ccl`
Expected: nested CCL output

Run: `echo '{"database": {"host": "db.example.com", "port": 5432}}' | cargo run -p sickle-cli -- convert --from json --to ccl | cargo run -p sickle-cli -- convert --from ccl --to json`
Expected: JSON round-trip (structure preserved, values may be stringified)

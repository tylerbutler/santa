# Architecture

Santa is a Rust-based package manager meta-tool that provides unified interfaces across different package managers. It focuses on safe script generation rather than direct command execution.

## Workspace Structure

The project is a Cargo workspace with three crates:

```
crates/
├── santa-cli/       # Main CLI application (binary)
├── santa-data/      # Data models and CCL configuration (library)
└── sickle/          # General-purpose CCL parser with Serde support (library)
```

### sickle (CCL Parser)

General-purpose CCL (Categorical Configuration Language) parser. Published independently to crates.io.

Sickle exposes a single round-tripping syntax tree: parsing keeps every node's source text, so an
unmodified document renders back byte-for-byte and only edited nodes are rewritten.

```
crates/sickle/src/
├── lib.rs          # Public API
├── lexer.rs        # Line-level CCL rules (delimiters, indentation, tabs, CRLF)
├── parser.rs       # Builds the syntax tree from lexed lines
├── encode.rs       # Renders the tree back to text
├── document.rs     # DocumentMut: the root, parsing, and rendering
├── table.rs        # Table, Array, entry APIs, CCL composition
├── item.rs         # Item: the node type
├── value.rs        # Value: CCL's single scalar kind
├── repr.rs         # Key, Decor, RawString (formatting metadata)
├── path.rs         # Checked path reads and edits
├── de.rs           # Serde deserializer (CCL string → Rust types)
├── ser.rs          # Serde serializer (Rust types → CCL string)
├── options.rs      # Parse-time and access-time behavior options
├── error.rs        # ParseError, GetError, EditError, and the unified Error
└── unstable.rs     # Flat-entry surface used by the CCL conformance suites
```

**Feature flags** control what functionality is compiled:
- *(default)* - Parsing, editing, and rendering
- `serde` - `sickle::de` and `sickle::ser`
- `intern` - String interning for very large documents
- `full` - Everything above
- `unstable` - Spec-compliance surface; not covered by semver

### santa-data (Data Models)

Reusable library for Santa's data structures and CCL-based configuration.

```
crates/santa-data/src/
├── lib.rs          # Library exports
├── models.rs       # Package, PackageManager, Platform types
├── parser.rs       # CCL-based package data parsing
├── config.rs       # Configuration management
└── schemas.rs      # Data validation schemas
```

### santa-cli (CLI Application)

Main binary with clap-based CLI, script generation, and package manager integrations.

```
crates/santa-cli/src/
├── main.rs             # Entry point, clap argument parsing
├── lib.rs              # Library exports
├── commands.rs         # Command dispatch
├── commands/           # Individual command implementations
├── script_generator.rs # Safe script generation with Tera templates
├── completions.rs      # Shell completion generation
├── configuration/      # CCL config with hot-reloading
├── sources/            # Package source abstractions
├── catalog.rs          # Package catalog management
├── data.rs             # Data layer coordination
├── data_layers.rs      # Layered data resolution
├── source_layers.rs    # Source priority ordering
├── plugins.rs          # Package manager plugin system
├── traits.rs           # Core trait definitions
├── errors.rs           # Unified error types
└── util.rs             # Shared utilities
```

## Key Design Decisions

### Script Generation Model

Santa generates platform-specific scripts instead of executing commands directly. This is the core security design:

- **Safe mode** (default): Generates `.sh`/`.ps1`/`.bat` scripts for user review
- **Execute mode**: Opt-in direct execution with the same generated scripts
- **Template-driven**: Tera templates in `templates/` define script formats

```
templates/
├── install.sh.tera     # Unix install script
├── install.ps1.tera    # PowerShell install script
├── check.sh.tera       # Unix check script
└── check.ps1.tera      # PowerShell check script
```

All user inputs are sanitized via `shell-escape` before template interpolation to prevent command injection.

### CCL Configuration

Santa uses CCL (Categorical Configuration Language) rather than TOML/YAML:

- Package data stored in `data/sources/*.ccl` (per-manager)
- Application configuration uses CCL format
- Transparent YAML-to-CCL migration for legacy configs
- Hot-reloading via file system watchers (`notify` crate)

### Layered Data Resolution

Package data is resolved through priority-ordered layers:

```
User overrides → Source files → Generated index → Defaults
```

### Async Architecture

- Standardized on `tokio` runtime with multi-threaded executor
- All subprocess execution via `tokio::process::Command`
- Shared state uses `tokio::sync::RwLock`
- Caching via `moka` with TTL and LRU eviction

### Error Handling

- Structured errors via `thiserror` (`SantaError` enum)
- Error chaining via `anyhow` for contextual information
- Graceful degradation with user-friendly messages

## Data Flow

### Package Installation

```
CLI args → resolve package name
    ↓
Load from layered data sources
    ↓
Detect platform + available package managers
    ↓
Generate install script from Tera template
    ↓
Safe mode: write script to disk
Execute mode: run script via shell
```

### Package Data Pipeline

```
External APIs (Homebrew, Scoop, AUR, Repology)
    ↓
collect-packages → raw API data
    ↓
crossref-packages → ranked candidates
    ↓
build-repology-cache → name mappings
    ↓
validate-cached → verified entries
    ↓
merge-verified → source CCL files
    ↓
generate-index → runtime known_packages.ccl
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed pipeline documentation.

## Testing Strategy

| Type | Location | Tool |
|------|----------|------|
| Unit tests | Embedded in source files | `cargo test` |
| Integration tests | `crates/*/tests/` | `cargo test` / `cargo nextest` |
| Property-based | `tests/property_tests.rs` | `proptest` |
| Security | `tests/security_tests.rs` | Custom assertions |
| CCL conformance | `crates/sickle/tests/` | ccl-test-data JSON suite |
| Benchmarks | `benches/` | `criterion` |

## Decisions

### Why Script Generation Over Direct Execution

Direct execution of package manager commands poses security risks (command injection via package names). Script generation:
- Makes the actual commands visible and auditable
- Allows user review before execution
- Prevents injection by design (template + escaped values)
- Supports offline/manual workflows

### Why CCL Over TOML

- More natural for package data (key-value with nesting)
- Supports comments with `/=` syntax
- Simpler syntax for the common case
- Dogfooding the sickle parser

### Why Workspace Architecture

- `sickle` is independently useful (published to crates.io)
- `santa-data` can be used by other tools
- Clear separation of parsing, data modeling, and CLI concerns
- Independent versioning and publishing

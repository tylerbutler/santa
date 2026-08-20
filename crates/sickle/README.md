# Sickle

A round-tripping parser and editor for **CCL** (Categorical Configuration Language), with optional
Serde support.

Sickle parses CCL into a mutable syntax tree that remembers its source text. Reading, editing, and
writing a document preserves comments, blank lines, key order, duplicate keys, indentation, and
spacing wherever they were not changed.

## Features

- **Lossless round-tripping** — an unmodified document renders back byte-for-byte
- **Two editing styles** — terse raw indexing, or checked path operations with structured errors
- **Full CCL support** — nested blocks, both list spellings, duplicate keys, multiline values,
  comments
- **Configurable behavior** — spacing, delimiter choice, tabs, CRLF, boolean strictness, list
  coercion
- **Optional Serde** — `sickle::de` and `sickle::ser`
- **Pure Rust** — no unsafe code

## Quick start

```rust
use sickle::DocumentMut;

let source = "/= service\nname = api\nport = 8080\n";
let mut doc: DocumentMut = source.parse().unwrap();

assert_eq!(doc.get_string(["name"]).unwrap(), "api");
assert_eq!(doc.get_int(["port"]).unwrap(), 8080);

doc.set_int(["port"], 9090).unwrap();
assert_eq!(doc.to_string(), "/= service\nname = api\nport = 9090\n");
```

## The tree

| Type                        | Role                                                                 |
| --------------------------- | -------------------------------------------------------------------- |
| `DocumentMut`               | The root: a `Table` plus the `Options` used to parse it              |
| `Item`                      | A node: `Value` (scalar), `Table` (block), `Array` (list), or `None` |
| `Table`                     | An ordered block; duplicate keys are legal and meaningful            |
| `Array`                     | A list, in either CCL spelling (`ArrayKind`)                         |
| `Value`                     | A scalar, with checked `as_integer` / `as_float` / `as_bool`         |
| `Key`, `Decor`, `RawString` | The formatting that makes round-tripping possible                    |

Build nodes with `value()`, `table()`, and `array()`.

## Raw versus checked editing

```rust
use sickle::{table, value, DocumentMut};

let mut doc = DocumentMut::new();

// Raw: terse, but indexing panics on a missing key and `IndexMut` auto-vivifies.
doc.as_table_mut().insert("server", table());
doc["server"]["host"] = value("localhost");

// Checked: takes a path, returns `GetError`/`EditError` naming the path that failed.
doc.set_int(["server", "port"], 8080).unwrap();
assert!(doc.get_string(["server", "missing"]).is_err());

assert_eq!(doc.to_string(), "server =\n  host = localhost\n  port = 8080");
```

Use the checked API for any path that comes from user input.

## Serde

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    name: String,
    hosts: Vec<String>,
}

let config: Config = sickle::de::from_str("name = api\nhosts =\n  = a\n  = b\n").unwrap();
assert_eq!(config.hosts, vec!["a", "b"]);
```

Serializing produces new text, so it cannot preserve comments. To keep them, parse the document
once and write the values you changed back through the checked API — see
`examples/serde_usage.rs`.

## CCL syntax

```ccl
/= This is a comment

name = MyApp
version = 1.0.0

/= Lists use bare `=` items ...
dependencies =
  = tokio
  = serde

/= ... or a repeated key
feature = tracing
feature = metrics

/= Nested configuration
database =
  host = localhost
  port = 5432
  credentials =
    username = admin
    password = secret
```

## How CCL differs from TOML

Sickle's API follows `toml_edit`'s shape, but the semantics are CCL's:

- **One scalar kind: text.** Numeric and boolean reads are checked views over that text, so the
  original spelling always round-trips.
- **No inline tables, arrays of tables, or date-times.** CCL cannot spell them.
- **Duplicate keys are legal** and read as a list; repeated blocks compose (`Table::get_composed`).
- **Two list spellings**, modelled by `ArrayKind`.
- **Comments and blank lines are trivia**, stored in `Decor`, so they never leak into iteration or
  Serde maps.

## Cargo features

| Feature     | Effect                                                                 |
| ----------- | ---------------------------------------------------------------------- |
| *(default)* | Parsing, editing, and rendering                                        |
| `serde`     | `sickle::de` and `sickle::ser`                                         |
| `intern`    | String interning for very large documents                              |
| `full`      | Everything above                                                       |
| `unstable`  | Spec-compliance surface for the CCL test suites; not covered by semver  |

## Migrating from 0.4

Sickle 0.5 replaces the overlapping 0.4 APIs with one syntax tree. `CclObject`, `CclReader`, the
flat `Entry`/`parse`/`build_hierarchy` pipeline, `CclPrinter`, and the merge-only `Document`
(`load_document`, `update_str`, `edit_str`) are gone.

| 0.4                                             | 0.5                                               |
| ----------------------------------------------- | ------------------------------------------------- |
| `sickle::load(text)`                            | `DocumentMut::parse(text)` or `text.parse()`      |
| `sickle::parse(text)`                           | `DocumentMut::parse(text)`                        |
| `model.get("k")`                                | `doc.get(["k"])` or `doc.as_table().get("k")`     |
| `model.get_string("k")`                         | `doc.get_string(["k"])`                           |
| `model.get_list("k")`                           | `doc.get_list(["k"])`                             |
| `CclPrinter::new().print(&model)`               | `doc.to_string()` (add `doc.fmt()` to normalize)  |
| `sickle::from_str` / `sickle::to_string`        | `sickle::de::from_str` / `sickle::ser::to_string` |
| `ParserOptions` + `BoolOptions` + `ListOptions` | `Options`                                         |
| `load_document` + `reserialize`                 | Parse once, then edit through the checked API     |
| features `hierarchy` / `printer` / `document`   | default (no feature needed)                       |

## License

MIT

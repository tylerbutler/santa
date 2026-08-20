//! # Sickle — a round-tripping CCL parser and editor
//!
//! Sickle parses [CCL](https://ccl.tylerbutler.com) into a mutable syntax tree
//! that remembers its source text. Reading, editing, and writing a document
//! preserves comments, blank lines, key order, duplicate keys, indentation, and
//! spacing wherever they were not changed.
//!
//! ```
//! use sickle::DocumentMut;
//!
//! let source = "/= service\nname = api\nport = 8080\n";
//! let mut doc: DocumentMut = source.parse().unwrap();
//!
//! assert_eq!(doc.get_string(["name"]).unwrap(), "api");
//! assert_eq!(doc.get_int(["port"]).unwrap(), 8080);
//!
//! doc.set_int(["port"], 9090).unwrap();
//! assert_eq!(doc.to_string(), "/= service\nname = api\nport = 9090\n");
//! ```
//!
//! ## The tree
//!
//! * [`DocumentMut`] is the root; it owns a [`Table`] and the [`Options`] used
//!   to parse it.
//! * [`Item`] is a node: [`Item::Value`] (a scalar), [`Item::Table`] (a nested
//!   block), [`Item::Array`] (a list), or [`Item::None`] (absent).
//! * [`Key`], [`Decor`], and [`RawString`] carry the formatting that makes
//!   round-tripping possible.
//!
//! Build nodes with [`value()`], [`table()`], and [`array()`].
//!
//! ## Raw versus checked editing
//!
//! Two styles are available, and they compose freely:
//!
//! * **Raw** — [`Table::get`], [`Table::insert`], [`Table::entry`], and
//!   `Index`/`IndexMut`. Terse, but indexing panics on a missing key and
//!   `IndexMut` auto-vivifies, exactly like `toml_edit`.
//! * **Checked** — [`DocumentMut::get_string`], [`DocumentMut::set_list`],
//!   [`DocumentMut::remove`], [`DocumentMut::insert_comment_before`], and
//!   friends. These take a path and return [`GetError`] or [`EditError`] with
//!   the full path that failed. Always prefer these for user-controlled paths.
//!
//! ```
//! use sickle::{table, value, DocumentMut};
//!
//! let mut doc = DocumentMut::new();
//!
//! // raw
//! doc.as_table_mut().insert("server", table());
//! doc["server"]["host"] = value("localhost");
//!
//! // checked
//! doc.set_int(["server", "port"], 8080).unwrap();
//! assert!(doc.get_string(["server", "missing"]).is_err());
//!
//! assert_eq!(doc.to_string(), "server =\n  host = localhost\n  port = 8080");
//! ```
//!
//! ## How CCL differs from TOML
//!
//! The shape of the API follows `toml_edit`, but the semantics are CCL's:
//!
//! * There is **one scalar kind**: text. [`Value::as_integer`],
//!   [`Value::as_float`], and [`Value::as_bool`] are checked views over that
//!   text, so numeric and boolean spelling always round-trips exactly.
//! * There are **no inline tables, no arrays of tables, and no date-times**.
//!   CCL cannot spell them, so Sickle does not pretend otherwise.
//! * **Duplicate keys are legal** and meaningful: they read as a list.
//! * **Lists have two spellings** — a bare-list block (`= item` lines) and a
//!   repeated key — modelled by [`ArrayKind`].
//! * **Comments (`/= text`) and blank lines are trivia**, stored in [`Decor`].
//!   They never appear as table keys, so iteration and Serde maps stay clean.
//! * Behavior that the CCL specification leaves open — spacing around `=`,
//!   delimiter choice, tabs, CRLF, boolean strictness, list coercion — is
//!   configured through [`Options`].
//!
//! ## Serde
//!
//! With the `serde` feature, [`de::from_str`] and [`ser::to_string`] map CCL to
//! and from Rust types. Typed editing composes explicitly: deserialize, mutate,
//! then write the values back through the checked API so comments survive.
//!
//! ## Cargo features
//!
//! * *(default)* — parsing, editing, and rendering.
//! * `serde` — [`de`] and [`ser`].
//! * `intern` — string interning for very large documents.
//! * `full` — everything above.
//! * `unstable` — a spec-compliance surface used by the CCL test suites. Not
//!   covered by semantic versioning.

#![warn(missing_docs)]

mod encode;
mod lexer;
mod parser;

pub mod document;
pub mod error;
pub mod item;
pub mod options;
pub mod path;
pub mod repr;
pub mod table;
pub mod value;

#[cfg(feature = "serde")]
pub mod de;
#[cfg(feature = "serde")]
pub mod ser;

#[cfg(feature = "unstable")]
pub mod unstable;

pub use document::{parse, parse_with, DocumentMut};
pub use error::{EditError, Error, ExpectedType, GetError, ParseError, Position, Result};
pub use item::{array, table, value, Item};
pub use options::{
    BoolBehavior, CrlfBehavior, DelimiterStrategy, ListBehavior, Options, SpacingBehavior,
    TabBehavior,
};
pub use path::{IntoPath, PathSegment};
pub use repr::{Decor, Key, RawString};
pub use table::{compose, Array, ArrayKind, Entry, OccupiedEntry, Table, VacantEntry};
pub use value::Value;

#[cfg(feature = "serde")]
pub use error::{DeserializeError, SerializeError};

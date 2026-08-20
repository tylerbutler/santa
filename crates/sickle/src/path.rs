//! Checked, path-based reads and edits.
//!
//! Raw access through [`Index`](std::ops::Index) is terse but panics or
//! auto-vivifies. Every user-controlled path should instead go through the
//! checked API in this module, which reports [`GetError`] and [`EditError`]
//! with the full path that failed.
//!
//! Paths are sequences of [`PathSegment`]s: borrowed strings select table keys
//! and integers select list elements. An array of `&str` is accepted directly
//! for the common all-keys case.
//!
//! ```
//! use sickle::{DocumentMut, PathSegment};
//!
//! let doc: DocumentMut = "server =\n  hosts =\n    = a\n    = b\n".parse().unwrap();
//! assert!(doc.get_string(["server", "hosts"]).is_err());
//! assert_eq!(
//!     doc.get_string([
//!         PathSegment::key("server"),
//!         PathSegment::key("hosts"),
//!         PathSegment::index(0),
//!     ])
//!     .unwrap(),
//!     "a"
//! );
//! ```

use crate::error::{EditError, ExpectedType, GetError};
use crate::item::Item;
use crate::options::Options;
use crate::repr::Key;
use crate::table::Table;
use std::fmt;
use std::str::FromStr;

/// One step of a path: a table key or a list index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSegment<'a> {
    /// Select a table key.
    Key(&'a str),
    /// Select a list element.
    Index(usize),
}

impl<'a> PathSegment<'a> {
    /// Select a table key.
    pub const fn key(key: &'a str) -> Self {
        PathSegment::Key(key)
    }

    /// Select a list element.
    pub const fn index(index: usize) -> Self {
        PathSegment::Index(index)
    }
}

impl fmt::Display for PathSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Key(key) => f.write_str(key),
            PathSegment::Index(index) => write!(f, "[{index}]"),
        }
    }
}

impl<'a> From<&'a str> for PathSegment<'a> {
    fn from(key: &'a str) -> Self {
        PathSegment::Key(key)
    }
}

impl<'a> From<&'a String> for PathSegment<'a> {
    fn from(key: &'a String) -> Self {
        PathSegment::Key(key.as_str())
    }
}

impl From<usize> for PathSegment<'_> {
    fn from(index: usize) -> Self {
        PathSegment::Index(index)
    }
}

/// Anything that can be used as a path.
///
/// Implemented for a single key, arrays and slices of keys, and arrays, slices,
/// and vectors of [`PathSegment`] for mixed key/index paths.
pub trait IntoPath<'a> {
    /// Convert into path segments.
    fn into_path(self) -> Vec<PathSegment<'a>>;
}

impl<'a> IntoPath<'a> for &'a str {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        vec![PathSegment::Key(self)]
    }
}

impl<'a> IntoPath<'a> for &'a String {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        vec![PathSegment::Key(self.as_str())]
    }
}

impl<'a, const N: usize> IntoPath<'a> for [&'a str; N] {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.into_iter().map(PathSegment::Key).collect()
    }
}

impl<'a> IntoPath<'a> for &'a [&'a str] {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.iter().copied().map(PathSegment::Key).collect()
    }
}

impl<'a> IntoPath<'a> for &'a [String] {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.iter().map(|k| PathSegment::Key(k.as_str())).collect()
    }
}

impl<'a> IntoPath<'a> for &'a Vec<String> {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.as_slice().into_path()
    }
}

impl<'a, const N: usize> IntoPath<'a> for [PathSegment<'a>; N] {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.to_vec()
    }
}

impl<'a> IntoPath<'a> for &[PathSegment<'a>] {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self.to_vec()
    }
}

impl<'a> IntoPath<'a> for Vec<PathSegment<'a>> {
    fn into_path(self) -> Vec<PathSegment<'a>> {
        self
    }
}

/// The node a lookup starts from.
///
/// Document reads start from a borrowed root [`Table`]; nested reads start from
/// an [`Item`]. Sharing one enum keeps a single implementation of the walk.
#[derive(Clone, Copy)]
pub(crate) enum Root<'a> {
    /// A tree node.
    Item(&'a Item),
    /// A document root table.
    Table(&'a Table),
}

impl<'a> Root<'a> {
    fn as_table(&self) -> Option<&'a Table> {
        match self {
            Root::Item(item) => item.as_table(),
            Root::Table(table) => Some(table),
        }
    }

    fn as_array(&self) -> Option<&'a crate::table::Array> {
        match self {
            Root::Item(item) => item.as_array(),
            Root::Table(_) => None,
        }
    }

    fn type_name(&self) -> ExpectedType {
        match self {
            Root::Item(item) => item.type_name(),
            Root::Table(_) => ExpectedType::Table,
        }
    }
}

/// Render a path for error messages.
pub(crate) fn display_path(path: &[PathSegment<'_>]) -> String {
    let mut out = String::new();
    for segment in path {
        match segment {
            PathSegment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            PathSegment::Index(index) => out.push_str(&format!("[{index}]")),
        }
    }
    out
}

fn step<'a>(
    current: Root<'a>,
    path: &[PathSegment<'_>],
    depth: usize,
) -> Result<&'a Item, GetError> {
    let so_far = display_path(&path[..=depth]);
    match &path[depth] {
        PathSegment::Key(key) => {
            let table = current.as_table().ok_or_else(|| GetError::TypeMismatch {
                path: display_path(&path[..depth]),
                expected: ExpectedType::Table,
                found: current.type_name(),
            })?;
            table.get(key).ok_or_else(|| GetError::MissingKey {
                path: so_far,
                key: (*key).to_string(),
            })
        }
        PathSegment::Index(index) => {
            let array = current.as_array().ok_or_else(|| GetError::TypeMismatch {
                path: display_path(&path[..depth]),
                expected: ExpectedType::List,
                found: current.type_name(),
            })?;
            array.get(*index).ok_or_else(|| GetError::IndexOutOfBounds {
                path: so_far,
                index: *index,
                len: array.len(),
            })
        }
    }
}

/// Follow `path` from `root`.
pub(crate) fn resolve<'a>(root: Root<'a>, path: &[PathSegment<'_>]) -> Result<&'a Item, GetError> {
    if path.is_empty() {
        return Err(GetError::EmptyPath);
    }
    let mut current = step(root, path, 0)?;
    for depth in 1..path.len() {
        current = step(Root::Item(current), path, depth)?;
    }
    Ok(current)
}

/// Follow `path` from a mutable root table.
pub(crate) fn resolve_in_table_mut<'a>(
    root: &'a mut Table,
    path: &[PathSegment<'_>],
) -> Result<&'a mut Item, GetError> {
    if path.is_empty() {
        return Err(GetError::EmptyPath);
    }
    let first = match &path[0] {
        PathSegment::Key(key) => root.get_mut(key).ok_or_else(|| GetError::MissingKey {
            path: display_path(&path[..1]),
            key: (*key).to_string(),
        })?,
        // A document root is always a block, so an index cannot start a path.
        PathSegment::Index(_) => {
            return Err(GetError::TypeMismatch {
                path: display_path(&path[..1]),
                expected: ExpectedType::List,
                found: ExpectedType::Table,
            })
        }
    };
    navigate_existing_mut_from(first, path, 1)
}

fn navigate_existing_mut_from<'i>(
    start: &'i mut Item,
    path: &[PathSegment<'_>],
    from: usize,
) -> Result<&'i mut Item, GetError> {
    let mut current = start;
    for depth in from..path.len() {
        let so_far = display_path(&path[..=depth]);
        let found = current.type_name();
        current = match &path[depth] {
            PathSegment::Key(key) => {
                let table = current
                    .as_table_mut()
                    .ok_or_else(|| GetError::TypeMismatch {
                        path: display_path(&path[..depth]),
                        expected: ExpectedType::Table,
                        found,
                    })?;
                table.get_mut(key).ok_or_else(|| GetError::MissingKey {
                    path: so_far,
                    key: (*key).to_string(),
                })?
            }
            PathSegment::Index(index) => {
                let array = current
                    .as_array_mut()
                    .ok_or_else(|| GetError::TypeMismatch {
                        path: display_path(&path[..depth]),
                        expected: ExpectedType::List,
                        found,
                    })?;
                let len = array.len();
                array.get_mut(*index).ok_or(GetError::IndexOutOfBounds {
                    path: so_far,
                    index: *index,
                    len,
                })?
            }
        };
    }
    Ok(current)
}

/// Walk every segment, creating missing tables along the way.
fn resolve_parent_mut<'i>(
    item: &'i mut Item,
    path: &[PathSegment<'_>],
) -> Result<&'i mut Item, EditError> {
    let mut current = item;
    for (depth, segment) in path.iter().enumerate() {
        let so_far = display_path(&path[..=depth]);
        current = match segment {
            PathSegment::Key(key) => {
                if current.is_none() {
                    *current = crate::item::table();
                }
                let table = current.as_table_mut().ok_or_else(|| EditError::NotATable {
                    path: display_path(&path[..depth]),
                })?;
                if !table.contains_key(key) {
                    table.append(Key::new(*key), crate::item::table());
                }
                table.get_mut(key).expect("just inserted")
            }
            PathSegment::Index(index) => {
                let array = current
                    .as_array_mut()
                    .ok_or_else(|| EditError::NotAnArray {
                        path: display_path(&path[..depth]),
                    })?;
                let len = array.len();
                array.get_mut(*index).ok_or(EditError::IndexOutOfBounds {
                    path: so_far,
                    index: *index,
                    len,
                })?
            }
        };
    }
    Ok(current)
}

/// Walk `path` without creating anything.
fn navigate_existing_mut<'i>(
    item: &'i mut Item,
    path: &[PathSegment<'_>],
) -> Result<&'i mut Item, EditError> {
    navigate_existing_mut_from(item, path, 0).map_err(|err| match err {
        GetError::MissingKey { path, key } => EditError::MissingKey { path, key },
        GetError::IndexOutOfBounds { path, index, len } => {
            EditError::IndexOutOfBounds { path, index, len }
        }
        GetError::TypeMismatch {
            path,
            expected: ExpectedType::List,
            ..
        } => EditError::NotAnArray { path },
        GetError::TypeMismatch { path, .. } => EditError::NotATable { path },
        GetError::EmptyPath => EditError::EmptyPath,
        GetError::InvalidValue { path, .. } => EditError::NotATable { path },
    })
}

/// Read a scalar string at `path`.
pub(crate) fn get_string<'a>(
    root: Root<'a>,
    path: &[PathSegment<'_>],
) -> Result<&'a str, GetError> {
    let node = resolve(root, path)?;
    node.as_str().ok_or_else(|| GetError::TypeMismatch {
        path: display_path(path),
        expected: ExpectedType::String,
        found: node.type_name(),
    })
}

/// Read an integer at `path`.
pub(crate) fn get_int(root: Root<'_>, path: &[PathSegment<'_>]) -> Result<i64, GetError> {
    let text = get_string(root, path)?;
    text.trim()
        .parse::<i64>()
        .map_err(|_| GetError::InvalidValue {
            path: display_path(path),
            expected: ExpectedType::Integer,
            value: text.to_string(),
        })
}

/// Read a float at `path`.
pub(crate) fn get_float(root: Root<'_>, path: &[PathSegment<'_>]) -> Result<f64, GetError> {
    let text = get_string(root, path)?;
    text.trim()
        .parse::<f64>()
        .map_err(|_| GetError::InvalidValue {
            path: display_path(path),
            expected: ExpectedType::Float,
            value: text.to_string(),
        })
}

/// Read a boolean at `path` using `options` for strictness.
pub(crate) fn get_bool(
    root: Root<'_>,
    path: &[PathSegment<'_>],
    options: &Options,
) -> Result<bool, GetError> {
    let text = get_string(root, path)?;
    options
        .parse_bool(text.trim())
        .ok_or_else(|| GetError::InvalidValue {
            path: display_path(path),
            expected: ExpectedType::Boolean,
            value: text.to_string(),
        })
}

/// Read any [`FromStr`] type at `path`.
pub(crate) fn value_get<T>(root: Root<'_>, path: &[PathSegment<'_>]) -> Result<T, GetError>
where
    T: FromStr,
{
    let text = get_string(root, path)?;
    text.trim()
        .parse::<T>()
        .map_err(|_| GetError::InvalidValue {
            path: display_path(path),
            expected: ExpectedType::Value,
            value: text.to_string(),
        })
}

/// Read a list of scalars at `path`.
///
/// A bare-list block and a repeated key both read as lists. When
/// [`ListBehavior::Coerce`](crate::ListBehavior::Coerce) is configured, a lone
/// scalar reads as a one-element list.
pub(crate) fn get_list(
    root: Root<'_>,
    path: &[PathSegment<'_>],
    options: &Options,
) -> Result<Vec<String>, GetError> {
    if let Some(values) = repeated_key_values(root, path)? {
        return Ok(values);
    }

    let node = resolve(root, path)?;
    match node {
        Item::Array(array) => array
            .as_strings()
            .map(|items| items.into_iter().map(str::to_string).collect())
            .ok_or_else(|| GetError::TypeMismatch {
                path: display_path(path),
                expected: ExpectedType::List,
                found: ExpectedType::Table,
            }),
        Item::Value(value) if options.coerces_lists() => Ok(vec![value.as_str().to_string()]),
        other => Err(GetError::TypeMismatch {
            path: display_path(path),
            expected: ExpectedType::List,
            found: other.type_name(),
        }),
    }
}

/// When the final segment names a key that occurs more than once, those
/// occurrences are the list.
fn repeated_key_values(
    root: Root<'_>,
    path: &[PathSegment<'_>],
) -> Result<Option<Vec<String>>, GetError> {
    let Some((last, parents)) = path.split_last() else {
        return Ok(None);
    };
    let PathSegment::Key(key) = last else {
        return Ok(None);
    };
    let parent = if parents.is_empty() {
        root
    } else {
        match resolve(root, parents) {
            Ok(item) => Root::Item(item),
            Err(_) => return Ok(None),
        }
    };
    let Some(table) = parent.as_table() else {
        return Ok(None);
    };
    if table.count(key) < 2 {
        return Ok(None);
    }
    let values: Option<Vec<String>> = table
        .get_all(key)
        .into_iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect();
    values.map(Some).ok_or_else(|| GetError::TypeMismatch {
        path: display_path(path),
        expected: ExpectedType::List,
        found: ExpectedType::Table,
    })
}

/// List the distinct keys of the table at `path`, or of `root` for an empty path.
pub(crate) fn table_keys(
    root: Root<'_>,
    path: &[PathSegment<'_>],
) -> Result<Vec<String>, GetError> {
    let table = if path.is_empty() {
        root.as_table().ok_or_else(|| GetError::TypeMismatch {
            path: String::new(),
            expected: ExpectedType::Table,
            found: root.type_name(),
        })?
    } else {
        let node = resolve(root, path)?;
        node.as_table().ok_or_else(|| GetError::TypeMismatch {
            path: display_path(path),
            expected: ExpectedType::Table,
            found: node.type_name(),
        })?
    };
    Ok(table
        .unique_keys()
        .into_iter()
        .map(str::to_string)
        .collect())
}

/// Write `new_item` at `path`, creating intermediate tables as needed.
pub(crate) fn set(
    item: &mut Item,
    path: &[PathSegment<'_>],
    new_item: Item,
) -> Result<(), EditError> {
    let (last, parents) = path.split_last().ok_or(EditError::EmptyPath)?;
    let parent = resolve_parent_mut(item, parents)?;
    match last {
        PathSegment::Key(key) => {
            if parent.is_none() {
                *parent = crate::item::table();
            }
            let table = parent.as_table_mut().ok_or_else(|| EditError::NotATable {
                path: display_path(parents),
            })?;
            table.insert(Key::new(*key), new_item);
            Ok(())
        }
        PathSegment::Index(index) => {
            let array = parent.as_array_mut().ok_or_else(|| EditError::NotAnArray {
                path: display_path(parents),
            })?;
            let len = array.len();
            match array.get_mut(*index) {
                Some(slot) => {
                    *slot = new_item;
                    Ok(())
                }
                None => Err(EditError::IndexOutOfBounds {
                    path: display_path(path),
                    index: *index,
                    len,
                }),
            }
        }
    }
}

/// Remove the node at `path`, returning it.
pub(crate) fn remove(item: &mut Item, path: &[PathSegment<'_>]) -> Result<Item, EditError> {
    let (last, parents) = path.split_last().ok_or(EditError::EmptyPath)?;
    let parent = navigate_existing_mut(item, parents)?;
    match last {
        PathSegment::Key(key) => {
            let table = parent.as_table_mut().ok_or_else(|| EditError::NotATable {
                path: display_path(parents),
            })?;
            table.remove(key).ok_or_else(|| EditError::MissingKey {
                path: display_path(path),
                key: (*key).to_string(),
            })
        }
        PathSegment::Index(index) => {
            let array = parent.as_array_mut().ok_or_else(|| EditError::NotAnArray {
                path: display_path(parents),
            })?;
            if *index >= array.len() {
                return Err(EditError::IndexOutOfBounds {
                    path: display_path(path),
                    index: *index,
                    len: array.len(),
                });
            }
            Ok(array.remove(*index))
        }
    }
}

/// Insert a comment line immediately above the entry at `path`.
pub(crate) fn insert_comment_before(
    item: &mut Item,
    path: &[PathSegment<'_>],
    text: &str,
) -> Result<(), EditError> {
    validate_comment(text)?;

    let (last, parents) = path.split_last().ok_or(EditError::EmptyPath)?;
    let PathSegment::Key(key) = last else {
        return Err(EditError::NotATable {
            path: display_path(path),
        });
    };

    let parent = navigate_existing_mut(item, parents)?;
    let table = parent.as_table_mut().ok_or_else(|| EditError::NotATable {
        path: display_path(parents),
    })?;
    let position = table.position(key).ok_or_else(|| EditError::MissingKey {
        path: display_path(path),
        key: (*key).to_string(),
    })?;
    apply_comment(table, position, text);
    Ok(())
}

/// Reject comment text that cannot be written as a single `/=` line.
pub(crate) fn validate_comment(text: &str) -> Result<(), EditError> {
    if text.contains('\n') || text.contains('\r') {
        return Err(EditError::InvalidComment {
            reason: "comment text must not contain line breaks".to_string(),
        });
    }
    Ok(())
}

/// Insert a blank line immediately above the entry at `path`.
pub(crate) fn insert_blank_line_before(
    item: &mut Item,
    path: &[PathSegment<'_>],
) -> Result<(), EditError> {
    let (last, parents) = path.split_last().ok_or(EditError::EmptyPath)?;
    let PathSegment::Key(key) = last else {
        return Err(EditError::NotATable {
            path: display_path(path),
        });
    };

    let parent = navigate_existing_mut(item, parents)?;
    let table = parent.as_table_mut().ok_or_else(|| EditError::NotATable {
        path: display_path(parents),
    })?;
    let position = table.position(key).ok_or_else(|| EditError::MissingKey {
        path: display_path(path),
        key: (*key).to_string(),
    })?;
    apply_trivia(table, position, None);
    Ok(())
}

/// Prepend a `/=` comment line to the decoration of the entry at `position`.
pub(crate) fn apply_comment(table: &mut Table, position: usize, text: &str) {
    apply_trivia(table, position, Some(text));
}

/// Prepend a trivia line — a comment when `text` is set, otherwise a blank
/// line — to the decoration of the entry at `position`.
pub(crate) fn apply_trivia(table: &mut Table, position: usize, text: Option<&str>) {
    // An entry with no recorded formatting is rendered canonically, so the
    // comment has to reproduce the indentation the renderer would have used.
    let sibling_indent = table.inferred_indent().unwrap_or_default();
    let opens_with_newline = table.opens_with_newline() || position > 0;

    let key = table
        .entry_key_mut(position)
        .expect("caller checked the position");
    let prefix = match key.decor().prefix().as_str() {
        Some(current) => {
            let current = current.to_string();
            let indent = match current.rfind('\n') {
                Some(at) => current[at + 1..].to_string(),
                None => current.clone(),
            };
            match text {
                Some(text) => format!("{current}/= {text}\n{indent}"),
                None => format!("{current}\n{indent}"),
            }
        }
        None => {
            let lead = if opens_with_newline { "\n" } else { "" };
            match text {
                Some(text) => format!("{lead}{sibling_indent}/= {text}\n{sibling_indent}"),
                None => format!("{lead}\n{sibling_indent}"),
            }
        }
    };
    key.decor_mut().set_prefix(prefix);
}

/// Checked, path-based access on any node of the tree.
impl Item {
    /// Read the node at `path`.
    pub fn get<'p, P: IntoPath<'p>>(&self, path: P) -> Result<&Item, GetError> {
        resolve(Root::Item(self), &path.into_path())
    }

    /// Mutably read the node at `path`.
    pub fn get_mut<'p, P: IntoPath<'p>>(&mut self, path: P) -> Result<&mut Item, GetError> {
        navigate_existing_mut_from(self, &path.into_path(), 0)
    }

    /// Read a scalar string at `path`.
    pub fn get_string<'p, P: IntoPath<'p>>(&self, path: P) -> Result<&str, GetError> {
        get_string(Root::Item(self), &path.into_path())
    }

    /// Read an integer at `path`.
    pub fn get_int<'p, P: IntoPath<'p>>(&self, path: P) -> Result<i64, GetError> {
        get_int(Root::Item(self), &path.into_path())
    }

    /// Read a float at `path`.
    pub fn get_float<'p, P: IntoPath<'p>>(&self, path: P) -> Result<f64, GetError> {
        get_float(Root::Item(self), &path.into_path())
    }

    /// Read a boolean at `path` using strict CCL rules.
    pub fn get_bool<'p, P: IntoPath<'p>>(&self, path: P) -> Result<bool, GetError> {
        self.get_bool_with(path, &Options::new())
    }

    /// Read a boolean at `path` using the supplied behavior options.
    pub fn get_bool_with<'p, P: IntoPath<'p>>(
        &self,
        path: P,
        options: &Options,
    ) -> Result<bool, GetError> {
        get_bool(Root::Item(self), &path.into_path(), options)
    }

    /// Read a list of scalars at `path`.
    pub fn get_list<'p, P: IntoPath<'p>>(&self, path: P) -> Result<Vec<String>, GetError> {
        self.get_list_with(path, &Options::new())
    }

    /// Read a list of scalars at `path` using the supplied behavior options.
    pub fn get_list_with<'p, P: IntoPath<'p>>(
        &self,
        path: P,
        options: &Options,
    ) -> Result<Vec<String>, GetError> {
        get_list(Root::Item(self), &path.into_path(), options)
    }

    /// Read any [`FromStr`] type at `path`.
    pub fn value_get<'p, T, P>(&self, path: P) -> Result<T, GetError>
    where
        T: FromStr,
        P: IntoPath<'p>,
    {
        value_get::<T>(Root::Item(self), &path.into_path())
    }

    /// The distinct keys of the table at `path`.
    pub fn table_keys<'p, P: IntoPath<'p>>(&self, path: P) -> Result<Vec<String>, GetError> {
        table_keys(Root::Item(self), &path.into_path())
    }

    /// Write `item` at `path`, creating intermediate tables as needed.
    pub fn set<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
        item: impl Into<Item>,
    ) -> Result<(), EditError> {
        set(self, &path.into_path(), item.into())
    }

    /// Write a string scalar at `path`.
    pub fn set_string<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
        value: impl Into<String>,
    ) -> Result<(), EditError> {
        self.set(path, crate::item::value(value.into()))
    }

    /// Write an integer scalar at `path`.
    pub fn set_int<'p, P: IntoPath<'p>>(&mut self, path: P, value: i64) -> Result<(), EditError> {
        self.set(path, crate::item::value(value))
    }

    /// Write a float scalar at `path`.
    pub fn set_float<'p, P: IntoPath<'p>>(&mut self, path: P, value: f64) -> Result<(), EditError> {
        self.set(path, crate::item::value(value))
    }

    /// Write a boolean scalar at `path`.
    pub fn set_bool<'p, P: IntoPath<'p>>(&mut self, path: P, value: bool) -> Result<(), EditError> {
        self.set(path, crate::item::value(value))
    }

    /// Replace the list at `path`.
    pub fn set_list<'p, P, I, S>(&mut self, path: P, values: I) -> Result<(), EditError>
    where
        P: IntoPath<'p>,
        I: IntoIterator<Item = S>,
        S: Into<crate::value::Value>,
    {
        let array: crate::table::Array =
            values.into_iter().map(|v| Item::Value(v.into())).collect();
        self.set(path, Item::Array(array))
    }

    /// Remove the node at `path`, returning it.
    pub fn remove<'p, P: IntoPath<'p>>(&mut self, path: P) -> Result<Item, EditError> {
        remove(self, &path.into_path())
    }

    /// Insert a `/=` comment line directly above the entry at `path`.
    pub fn insert_comment_before<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
        text: &str,
    ) -> Result<(), EditError> {
        insert_comment_before(self, &path.into_path(), text)
    }

    /// Insert a blank line directly above the entry at `path`.
    pub fn insert_blank_line_before<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
    ) -> Result<(), EditError> {
        insert_blank_line_before(self, &path.into_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::value;
    use crate::DocumentMut;

    #[test]
    fn path_display_mixes_keys_and_indices() {
        let path = [
            PathSegment::key("a"),
            PathSegment::index(2),
            PathSegment::key("b"),
        ];
        assert_eq!(display_path(&path), "a[2].b");
    }

    #[test]
    fn missing_keys_report_the_full_path() {
        let doc: DocumentMut = "server =\n  host = h\n".parse().unwrap();
        let err = doc.get_string(["server", "port"]).unwrap_err();
        assert!(matches!(err, GetError::MissingKey { .. }));
        assert!(err.to_string().contains("server.port"));
    }

    #[test]
    fn type_mismatch_reports_expected_and_found() {
        let doc: DocumentMut = "server =\n  host = h\n".parse().unwrap();
        let err = doc.get_string(["server"]).unwrap_err();
        match err {
            GetError::TypeMismatch {
                expected, found, ..
            } => {
                assert_eq!(expected, ExpectedType::String);
                assert_eq!(found, ExpectedType::Table);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn empty_paths_are_rejected() {
        let doc = DocumentMut::new();
        let empty: [PathSegment<'_>; 0] = [];
        assert!(matches!(doc.get(empty), Err(GetError::EmptyPath)));
    }

    #[test]
    fn set_creates_intermediate_tables() {
        let mut doc = DocumentMut::new();
        doc.set_string(["a", "b", "c"], "x").unwrap();
        assert_eq!(doc.get_string(["a", "b", "c"]).unwrap(), "x");
        assert_eq!(doc.to_string(), "a =\n  b =\n    c = x");
    }

    #[test]
    fn set_through_a_scalar_fails() {
        let mut doc: DocumentMut = "a = 1\n".parse().unwrap();
        let err = doc.set_string(["a", "b"], "x").unwrap_err();
        assert!(matches!(err, EditError::NotATable { .. }));
    }

    #[test]
    fn remove_reports_missing_keys() {
        let mut doc: DocumentMut = "a = 1\n".parse().unwrap();
        assert!(doc.remove(["b"]).is_err());
        assert!(doc.remove(["a"]).is_ok());
    }

    #[test]
    fn comments_reject_line_breaks() {
        let mut doc: DocumentMut = "a = 1\n".parse().unwrap();
        let err = doc.insert_comment_before(["a"], "one\ntwo").unwrap_err();
        assert!(matches!(err, EditError::InvalidComment { .. }));
    }

    #[test]
    fn index_paths_walk_lists() {
        let doc: DocumentMut = "hosts =\n  = a\n  = b\n".parse().unwrap();
        assert_eq!(
            doc.get_string([PathSegment::key("hosts"), PathSegment::index(1)])
                .unwrap(),
            "b"
        );
        let err = doc
            .get_string([PathSegment::key("hosts"), PathSegment::index(5)])
            .unwrap_err();
        assert!(matches!(err, GetError::IndexOutOfBounds { .. }));
    }

    #[test]
    fn repeated_keys_read_as_lists() {
        let doc: DocumentMut = "item = a\nitem = b\n".parse().unwrap();
        assert_eq!(doc.get_list(["item"]).unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn set_list_replaces_values() {
        let mut doc: DocumentMut = "hosts =\n  = a\n".parse().unwrap();
        doc.set_list(["hosts"], ["x", "y"]).unwrap();
        assert_eq!(doc.get_list(["hosts"]).unwrap(), vec!["x", "y"]);
        assert!(doc.get(["hosts"]).unwrap().is_array());
    }

    #[test]
    fn generic_value_get_parses() {
        let doc: DocumentMut = "port = 8080\n".parse().unwrap();
        let port: u16 = doc.value_get(["port"]).unwrap();
        assert_eq!(port, 8080);
        assert!(doc.value_get::<u8, _>(["port"]).is_err());
    }

    #[test]
    fn item_level_paths_work_too() {
        let doc: DocumentMut = "server =\n  host = h\n".parse().unwrap();
        let server = doc.get(["server"]).unwrap();
        assert_eq!(server.get_string(["host"]).unwrap(), "h");

        let mut root = crate::item::table();
        root.set(["x"], value("1")).unwrap();
        assert_eq!(root.get_string(["x"]).unwrap(), "1");
    }

    #[test]
    fn table_keys_lists_distinct_keys() {
        let doc: DocumentMut = "a = 1\nb = 2\na = 3\n".parse().unwrap();
        assert_eq!(doc.root_keys(), vec!["a", "b"]);
    }
}

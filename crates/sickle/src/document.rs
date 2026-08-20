//! The document root: parsing, rendering, and top-level access.

use crate::encode;
use crate::error::{EditError, GetError, ParseError};
use crate::item::Item;
use crate::options::Options;
use crate::parser;
use crate::path::{apply_comment, IntoPath, Root};
use crate::repr::RawString;
use crate::table::Table;
use crate::value::Value;
use std::fmt;
use std::ops::{Index, IndexMut};
use std::str::FromStr;

/// A parsed CCL document that can be edited and rendered back to text.
///
/// `DocumentMut` owns the root [`Table`] plus the [`Options`] that were used to
/// parse it, so typed reads keep honoring the same boolean and list behaviors.
///
/// Parsing preserves everything: comments, blank lines, key order, duplicate
/// keys, indentation, spacing around `=`, and whether the file ended with a
/// newline. An unmodified document renders back byte-for-byte; edited or newly
/// created nodes render canonically.
///
/// ```
/// use sickle::{value, DocumentMut};
///
/// let source = "/= service config\nname = api\nport = 8080\n";
/// let mut doc: DocumentMut = source.parse().unwrap();
/// assert_eq!(doc.to_string(), source);
///
/// doc.set_int(["port"], 9090).unwrap();
/// assert_eq!(
///     doc.to_string(),
///     "/= service config\nname = api\nport = 9090\n"
/// );
///
/// doc.as_table_mut().insert("debug", value(true));
/// assert!(doc.to_string().ends_with("debug = true\n"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct DocumentMut {
    root: Table,
    options: Options,
}

impl DocumentMut {
    /// Create an empty document with default behaviors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse CCL text with default behaviors.
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Self::parse_with(input, &Options::new())
    }

    /// Parse CCL text with explicit behaviors.
    pub fn parse_with(input: &str, options: &Options) -> Result<Self, ParseError> {
        let root = parser::parse(input, options)?;
        Ok(Self {
            root,
            options: options.clone(),
        })
    }

    /// The behaviors used for parsing and typed reads.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Change the behaviors used by typed reads.
    pub fn set_options(&mut self, options: Options) {
        self.options = options;
    }

    /// The root table.
    pub fn as_table(&self) -> &Table {
        &self.root
    }

    /// Mutable access to the root table.
    pub fn as_table_mut(&mut self) -> &mut Table {
        &mut self.root
    }

    /// Consume the document, returning its root table.
    pub fn into_table(self) -> Table {
        self.root
    }

    /// The root as an [`Item`], for uniform tree walking.
    pub fn as_item(&self) -> Item {
        Item::Table(self.root.clone())
    }

    /// The number of top-level entries, counting duplicate keys separately.
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// Whether the document has no entries.
    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    /// Iterate over the top-level entries in source order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Item)> {
        self.root.iter()
    }

    /// Trivia after the last entry, including the document's final newline.
    pub fn trailing(&self) -> &RawString {
        self.root.trailing()
    }

    /// Replace the trivia after the last entry.
    pub fn set_trailing(&mut self, trailing: impl Into<RawString>) {
        self.root.set_trailing(trailing);
    }

    /// Canonicalize document syntax while retaining comments and blank lines.
    pub fn fmt(&mut self) {
        self.root.fmt();
    }

    // ----- checked path API -------------------------------------------------

    /// Read the node at `path`.
    pub fn get<'p, P: IntoPath<'p>>(&self, path: P) -> Result<&Item, GetError> {
        crate::path::resolve(Root::Table(&self.root), &path.into_path())
    }

    /// Mutably read the node at `path`.
    pub fn get_mut<'p, P: IntoPath<'p>>(&mut self, path: P) -> Result<&mut Item, GetError> {
        crate::path::resolve_in_table_mut(&mut self.root, &path.into_path())
    }

    /// Read a scalar string at `path`.
    pub fn get_string<'p, P: IntoPath<'p>>(&self, path: P) -> Result<&str, GetError> {
        crate::path::get_string(Root::Table(&self.root), &path.into_path())
    }

    /// Read an integer at `path`.
    pub fn get_int<'p, P: IntoPath<'p>>(&self, path: P) -> Result<i64, GetError> {
        crate::path::get_int(Root::Table(&self.root), &path.into_path())
    }

    /// Read a float at `path`.
    pub fn get_float<'p, P: IntoPath<'p>>(&self, path: P) -> Result<f64, GetError> {
        crate::path::get_float(Root::Table(&self.root), &path.into_path())
    }

    /// Read a boolean at `path`, honoring the document's boolean behavior.
    pub fn get_bool<'p, P: IntoPath<'p>>(&self, path: P) -> Result<bool, GetError> {
        crate::path::get_bool(Root::Table(&self.root), &path.into_path(), &self.options)
    }

    /// Read a list of scalars at `path`, honoring the document's list behavior.
    pub fn get_list<'p, P: IntoPath<'p>>(&self, path: P) -> Result<Vec<String>, GetError> {
        crate::path::get_list(Root::Table(&self.root), &path.into_path(), &self.options)
    }

    /// Read any [`FromStr`] type at `path`.
    pub fn value_get<'p, T, P>(&self, path: P) -> Result<T, GetError>
    where
        T: FromStr,
        P: IntoPath<'p>,
    {
        crate::path::value_get::<T>(Root::Table(&self.root), &path.into_path())
    }

    /// The distinct keys of the table at `path`, or of the root for an empty path.
    pub fn table_keys<'p, P: IntoPath<'p>>(&self, path: P) -> Result<Vec<String>, GetError> {
        crate::path::table_keys(Root::Table(&self.root), &path.into_path())
    }

    /// The distinct top-level keys, in order of first appearance.
    pub fn root_keys(&self) -> Vec<String> {
        self.root
            .unique_keys()
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// Write `item` at `path`, creating intermediate tables as needed.
    pub fn set<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
        item: impl Into<Item>,
    ) -> Result<(), EditError> {
        self.edit_root(path, |root, path| crate::path::set(root, path, item.into()))
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
        S: Into<Value>,
    {
        let array: crate::table::Array =
            values.into_iter().map(|v| Item::Value(v.into())).collect();
        self.set(path, Item::Array(array))
    }

    /// Remove the node at `path`, returning it.
    pub fn remove<'p, P: IntoPath<'p>>(&mut self, path: P) -> Result<Item, EditError> {
        self.edit_root(path, crate::path::remove)
    }

    /// Insert a `/=` comment line directly above the entry at `path`.
    ///
    /// ```
    /// use sickle::DocumentMut;
    ///
    /// let mut doc: DocumentMut = "name = api\nport = 80\n".parse().unwrap();
    /// doc.insert_comment_before(["port"], "listening port").unwrap();
    /// assert_eq!(doc.to_string(), "name = api\n/= listening port\nport = 80\n");
    /// ```
    pub fn insert_comment_before<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
        text: &str,
    ) -> Result<(), EditError> {
        self.edit_root(path, |root, path| {
            crate::path::insert_comment_before(root, path, text)
        })
    }

    /// Insert a blank line directly above the entry at `path`.
    pub fn insert_blank_line_before<'p, P: IntoPath<'p>>(
        &mut self,
        path: P,
    ) -> Result<(), EditError> {
        self.edit_root(path, crate::path::insert_blank_line_before)
    }

    /// Insert a blank line above the top-level entry at `position`.
    pub fn insert_blank_line_at(&mut self, position: usize) -> Result<(), EditError> {
        if position >= self.root.len() {
            return Err(EditError::IndexOutOfBounds {
                path: String::new(),
                index: position,
                len: self.root.len(),
            });
        }
        crate::path::apply_trivia(&mut self.root, position, None);
        Ok(())
    }

    /// Append a standalone `/=` comment line at the end of the document.
    pub fn push_comment(&mut self, text: &str) -> Result<(), EditError> {
        crate::path::validate_comment(text)?;
        let existing = self.root.trailing().or("").to_string();
        let lead = if self.root.is_empty() && existing.is_empty() {
            String::new()
        } else if existing.is_empty() {
            "\n".to_string()
        } else if existing.ends_with('\n') {
            existing
        } else {
            format!("{existing}\n")
        };
        self.root.set_trailing(format!("{lead}/= {text}\n"));
        Ok(())
    }

    /// Insert a `/=` comment line above the top-level entry at `position`.
    pub fn insert_comment_at(&mut self, position: usize, text: &str) -> Result<(), EditError> {
        crate::path::validate_comment(text)?;
        if position >= self.root.len() {
            return Err(EditError::IndexOutOfBounds {
                path: String::new(),
                index: position,
                len: self.root.len(),
            });
        }
        apply_comment(&mut self.root, position, text);
        Ok(())
    }

    /// Append a blank line at the end of the document.
    pub fn push_blank_line(&mut self) {
        let existing = self.root.trailing().or("").to_string();
        let lead = if self.root.is_empty() && existing.is_empty() {
            String::new()
        } else if existing.is_empty() {
            "\n".to_string()
        } else if existing.ends_with('\n') {
            existing
        } else {
            format!("{existing}\n")
        };
        self.root.set_trailing(format!("{lead}\n"));
    }

    /// Run a checked edit against the root, which is temporarily moved out of
    /// the document so the edit helpers can take `&mut Item`.
    fn edit_root<'p, P, T, F>(&mut self, path: P, edit: F) -> Result<T, EditError>
    where
        P: IntoPath<'p>,
        F: FnOnce(&mut Item, &[crate::path::PathSegment<'p>]) -> Result<T, EditError>,
    {
        let path = path.into_path();
        let mut root = Item::Table(std::mem::take(&mut self.root));
        let result = edit(&mut root, &path);
        self.restore(root);
        result
    }

    fn restore(&mut self, root: Item) {
        self.root = match root {
            Item::Table(table) => table,
            _ => Table::new(),
        };
    }
}

impl FromStr for DocumentMut {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DocumentMut::parse(s)
    }
}

impl fmt::Display for DocumentMut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&encode::render_document(&self.root))
    }
}

impl From<Table> for DocumentMut {
    fn from(root: Table) -> Self {
        Self {
            root,
            options: Options::new(),
        }
    }
}

impl Index<&str> for DocumentMut {
    type Output = Item;

    /// Panics when `key` is absent; use [`get`](DocumentMut::get) instead.
    fn index(&self, key: &str) -> &Item {
        &self.root[key]
    }
}

impl IndexMut<&str> for DocumentMut {
    /// Auto-vivifies `key` with [`Item::None`] when absent.
    fn index_mut(&mut self, key: &str) -> &mut Item {
        &mut self.root[key]
    }
}

/// Parse CCL text into a [`DocumentMut`] with default behaviors.
pub fn parse(input: &str) -> Result<DocumentMut, ParseError> {
    DocumentMut::parse(input)
}

/// Parse CCL text into a [`DocumentMut`] with explicit behaviors.
pub fn parse_with(input: &str, options: &Options) -> Result<DocumentMut, ParseError> {
    DocumentMut::parse_with(input, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::value;

    #[test]
    fn round_trips_unmodified_sources() {
        for source in [
            "",
            "a = 1",
            "a = 1\n",
            "/= comment\n\na = 1\nb =\n  c = 2\n",
            "list =\n  = one\n  = two\n\n/= trailing comment\n",
            "dup = 1\ndup = 2\n",
        ] {
            let doc = DocumentMut::parse(source).unwrap();
            assert_eq!(doc.to_string(), source, "source: {source:?}");
        }
    }

    #[test]
    fn editing_preserves_surrounding_trivia() {
        let source = "/= header\n\nname = api\n/= about port\nport = 80\n\n/= footer\n";
        let mut doc = DocumentMut::parse(source).unwrap();
        doc.set_int(["port"], 8080).unwrap();
        assert_eq!(
            doc.to_string(),
            "/= header\n\nname = api\n/= about port\nport = 8080\n\n/= footer\n"
        );
    }

    #[test]
    fn new_entries_are_appended_canonically() {
        let mut doc = DocumentMut::parse("a = 1\n").unwrap();
        doc.as_table_mut().insert("b", value("2"));
        assert_eq!(doc.to_string(), "a = 1\nb = 2\n");
    }

    #[test]
    fn removing_a_key_drops_its_line() {
        let mut doc = DocumentMut::parse("a = 1\nb = 2\nc = 3\n").unwrap();
        doc.remove(["b"]).unwrap();
        assert_eq!(doc.to_string(), "a = 1\nc = 3\n");
    }

    #[test]
    fn from_str_and_display_are_inverses() {
        let source = "x =\n  y = 1\n";
        let doc: DocumentMut = source.parse().unwrap();
        assert_eq!(doc.to_string(), source);
    }

    #[test]
    fn indexing_reads_and_writes() {
        let mut doc = DocumentMut::parse("a = 1\n").unwrap();
        assert_eq!(doc["a"].as_str(), Some("1"));
        doc["b"]["c"] = value("2");
        assert_eq!(doc.get_string(["b", "c"]).unwrap(), "2");
    }
}

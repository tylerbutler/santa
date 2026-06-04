//! Comment- and format-preserving document editing for CCL (issue #206).
//!
//! The serde round-trip (`from_str` -> edit struct -> `to_string`) is lossy: it
//! drops every comment, blank line, and formatting nuance that the typed struct
//! does not model. This module adds a [`Document`] type — analogous to
//! `toml_edit`'s `DocumentMut` — that retains the source trivia so a
//! read-modify-write cycle preserves the user's hand-written comments, blank
//! lines, key order, and spacing wherever the data is unchanged:
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Config { name: String, version: String }
//!
//! let text = "/= my config\nname = app\nversion = 1.0.0\n";
//! let doc = sickle::load_document(text).unwrap();
//! let mut cfg: Config = doc.deserialize().unwrap();
//! cfg.version = "2.0.0".to_string();
//! let out = doc.reserialize(&cfg).unwrap();
//! assert!(out.contains("/= my config"));   // comment preserved
//! assert!(out.contains("version = 2.0.0")); // value updated
//! ```
//!
//! ## How it works
//!
//! CCL parsing is recursive ("pacman"): a block is a list of `key = value`
//! entries, and each value is itself a block parsed the same way. [`Document`]
//! mirrors that structure as a recursive AST in which every node keeps its
//! **verbatim source lines**, plus blank lines and comments as first-class
//! items. Unchanged regions are emitted byte-for-byte from their original lines;
//! only edited entries are re-rendered from the freshly serialized struct.
//!
//! [`reserialize`](Document::reserialize) serializes the edited value to
//! canonical CCL, parses it into the same AST shape, and merges it into the base
//! document: matching keys keep their original formatting (recursing into nested
//! blocks), changed scalars are replaced, removed keys drop along with the
//! comments attached directly above them, and newly added keys are appended.

use crate::error::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::VecDeque;

use indexmap::IndexMap;

/// A CCL document that preserves comments, blank lines, key order, and
/// formatting across a read-modify-write cycle.
///
/// Construct one with [`load_document`], read a typed view with
/// [`deserialize`](Document::deserialize), and write the edited value back with
/// [`reserialize`](Document::reserialize).
#[derive(Debug, Clone)]
pub struct Document {
    /// The original source text, used for the typed (`deserialize`) view.
    source: String,
    /// Whether the source ended with a trailing newline.
    trailing_newline: bool,
    /// The top-level block of the document.
    items: Vec<Item>,
}

/// One line-level item within a block.
#[derive(Debug, Clone)]
enum Item {
    /// A blank line.
    Blank,
    /// A comment line, stored verbatim (including indentation), e.g. `"  /= note"`.
    Comment(String),
    /// A `key = value` entry (possibly with a nested block as its value).
    Entry(EntryNode),
}

/// A single CCL entry and its (possibly nested) value, kept as verbatim lines.
#[derive(Debug, Clone)]
struct EntryNode {
    /// The entry key (text before the first `=`, trimmed). Empty for bare list
    /// items (`= item`).
    key: String,
    /// The verbatim header line, e.g. `"  github ="` or `"name = app"`.
    header: String,
    /// Verbatim continuation lines (the indented child block / multiline value).
    child_lines: Vec<String>,
    /// Parsed children, present only when the value is structurally a CCL block
    /// (contains at least one `=` entry). `None` for scalar / multiline-string
    /// values.
    children: Option<Vec<Item>>,
}

/// Load CCL text into a [`Document`], retaining comments, blank lines, and order.
///
/// Requires the `document` feature.
pub fn load_document(input: &str) -> Result<Document> {
    let trailing_newline = input.ends_with('\n');
    // Work on logical lines without trailing '\n'. `lines()` also strips '\r',
    // which we keep out of the comparison; CRLF inputs round-trip via LF.
    let lines: Vec<&str> = input.lines().collect();
    let level = base_indent(&lines).unwrap_or(0);
    let items = parse_block(&lines, level);
    Ok(Document {
        source: input.to_string(),
        trailing_newline,
        items,
    })
}

impl Document {
    /// Deserialize the document into a typed value.
    ///
    /// Comments and blank lines are ignored, so this is equivalent to
    /// [`from_str`](crate::from_str) on the original source.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T> {
        crate::from_str(&self.source)
    }

    /// Serialize `value` back to CCL text, preserving the document's comments,
    /// blank lines, and formatting wherever the data is unchanged.
    ///
    /// Entries whose values are unchanged are emitted byte-for-byte from the
    /// original source. Changed scalar values are replaced, nested blocks are
    /// merged recursively, removed keys are dropped together with the comments
    /// attached directly above them, and newly added keys are appended.
    pub fn reserialize<T: Serialize>(&self, value: &T) -> Result<String> {
        let new_text = crate::to_string(value)?;
        let new_lines: Vec<&str> = new_text.lines().collect();
        let new_level = base_indent(&new_lines).unwrap_or(0);
        let new_items = parse_block(&new_lines, new_level);

        let merged = merge_block(&self.items, &new_items);
        let rendered = render_items(&merged);
        let mut out = rendered.join("\n");
        if self.trailing_newline && !out.is_empty() {
            out.push('\n');
        }
        Ok(out)
    }

    /// Render the document back to text without applying any edits.
    ///
    /// For an unmodified document this reproduces the original source. (Also
    /// available via [`Display`](std::fmt::Display)/`to_string`.)
    pub fn render(&self) -> String {
        let mut out = render_items(&self.items).join("\n");
        if self.trailing_newline && !out.is_empty() {
            out.push('\n');
        }
        out
    }
}

// ============================================================================
// Parsing: recursive, line-based, trivia-preserving
// ============================================================================

/// Number of leading whitespace characters on a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The indentation level of the first non-blank line, if any.
fn base_indent(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| indent_of(l))
}

/// Whether the next non-blank line at or after `start` is indented deeper than
/// `level` (i.e. still part of the current entry's value).
fn next_nonblank_deeper(lines: &[&str], start: usize, level: usize) -> bool {
    lines[start..]
        .iter()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| indent_of(l) > level)
}

/// The key portion of an entry header (text before the first `=`, trimmed).
fn split_key(header_trimmed: &str) -> String {
    match header_trimmed.find('=') {
        Some(p) => header_trimmed[..p].trim().to_string(),
        None => header_trimmed.trim().to_string(),
    }
}

/// The value portion of an entry header (text after the first `=`, trimmed).
fn inline_value(header: &str) -> &str {
    match header.find('=') {
        Some(p) => header[p + 1..].trim(),
        None => "",
    }
}

/// Parse a block of lines at the given indentation level into ordered items.
fn parse_block(lines: &[&str], level: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            items.push(Item::Blank);
            i += 1;
            continue;
        }

        if trimmed.starts_with("/=") {
            items.push(Item::Comment(line.to_string()));
            i += 1;
            continue;
        }

        // Entry header on this line; collect its continuation (child) lines:
        // every following line indented deeper than `level`, including interior
        // blank lines that still precede deeper content.
        let mut j = i + 1;
        while j < lines.len() {
            let l2 = lines[j];
            if l2.trim().is_empty() {
                if next_nonblank_deeper(lines, j + 1, level) {
                    j += 1;
                    continue;
                }
                break;
            }
            if indent_of(l2) > level {
                j += 1;
            } else {
                break;
            }
        }

        let child_lines: Vec<String> = lines[i + 1..j].iter().map(|s| s.to_string()).collect();
        let children = parse_children(&child_lines);
        items.push(Item::Entry(EntryNode {
            key: split_key(trimmed),
            header: line.to_string(),
            child_lines,
            children,
        }));
        i = j;
    }
    items
}

/// Parse child lines into a nested block, returning `Some` only when the value
/// is structurally a CCL block (has at least one `=` entry). Scalar and
/// multiline-string values return `None`.
fn parse_children(child_lines: &[String]) -> Option<Vec<Item>> {
    let refs: Vec<&str> = child_lines.iter().map(String::as_str).collect();
    let level = base_indent(&refs)?;
    let items = parse_block(&refs, level);
    let is_block = items
        .iter()
        .any(|it| matches!(it, Item::Entry(e) if e.header.contains('=')));
    is_block.then_some(items)
}

// ============================================================================
// Merging: splice the freshly serialized value into the base document
// ============================================================================

/// Merge a freshly serialized block (`new`, no trivia) into the base block
/// (`base`, with comments/blanks), preserving trivia and original formatting.
fn merge_block(base: &[Item], new: &[Item]) -> Vec<Item> {
    // Occurrence queues so duplicate keys (Vec fields, bare list items) match
    // positionally instead of collapsing.
    let mut queues: IndexMap<String, VecDeque<usize>> = IndexMap::new();
    for (idx, it) in new.iter().enumerate() {
        if let Item::Entry(e) = it {
            queues.entry(e.key.clone()).or_default().push_back(idx);
        }
    }
    let mut consumed = vec![false; new.len()];

    let mut result: Vec<Item> = Vec::new();
    // Comments/blanks seen since the last emitted entry, pending attachment.
    let mut trivia: Vec<Item> = Vec::new();

    for item in base {
        match item {
            Item::Blank | Item::Comment(_) => trivia.push(item.clone()),
            Item::Entry(b) => {
                let matched = queues.get_mut(&b.key).and_then(VecDeque::pop_front);
                match matched {
                    Some(ni) => {
                        consumed[ni] = true;
                        result.append(&mut trivia);
                        let n = match &new[ni] {
                            Item::Entry(e) => e,
                            _ => unreachable!("queues only index entries"),
                        };
                        result.push(Item::Entry(merge_entry(b, n)));
                    }
                    None => {
                        // Key removed by the edit: keep standalone trivia but drop
                        // the comments attached directly above this entry.
                        let mut kept = trivia_on_removal(&trivia);
                        result.append(&mut kept);
                        trivia.clear();
                    }
                }
            }
        }
    }
    // Trailing trivia (after the last entry) is always preserved.
    result.append(&mut trivia);

    // Append entries that exist only in the new value, in serializer order.
    // Skip empty-valued leftovers (empty inline value and no nested block): these
    // come from empty collections / `None`-like fields and would add spurious
    // `key =` lines that were never in the source.
    for (idx, it) in new.iter().enumerate() {
        if !consumed[idx] {
            if let Item::Entry(e) = it {
                if inline_value(&e.header).is_empty() && e.child_lines.is_empty() {
                    continue;
                }
                result.push(it.clone());
            }
        }
    }

    result
}

/// Merge a base entry with its matching new entry.
fn merge_entry(b: &EntryNode, n: &EntryNode) -> EntryNode {
    let b_block = b.children.is_some() && b.header.contains('=');
    let n_block = n.children.is_some();

    if b_block && n_block {
        // Same key, both nested: keep the base header's formatting and merge the
        // child blocks recursively.
        let merged = merge_block(b.children.as_ref().unwrap(), n.children.as_ref().unwrap());
        let child_lines = render_items(&merged);
        return EntryNode {
            key: b.key.clone(),
            header: b.header.clone(),
            child_lines,
            children: Some(merged),
        };
    }

    // Scalar value (or a structural change between scalar and block): keep the
    // base verbatim when the value is unchanged, otherwise take the new canonical
    // rendering.
    if entries_value_equal(b, n) {
        b.clone()
    } else {
        n.clone()
    }
}

/// Whether two entries encode the same value (ignoring formatting differences
/// such as spacing around `=`).
fn entries_value_equal(b: &EntryNode, n: &EntryNode) -> bool {
    inline_value(&b.header) == inline_value(&n.header)
        && b.child_lines.len() == n.child_lines.len()
        && b.child_lines
            .iter()
            .zip(&n.child_lines)
            .all(|(x, y)| x.trim_end() == y.trim_end())
}

/// Resolve pending trivia when the following entry is removed: keep everything
/// up to and including the last blank line (standalone comment blocks and
/// separators), and drop the comments attached directly above the removed entry.
fn trivia_on_removal(trivia: &[Item]) -> Vec<Item> {
    match trivia.iter().rposition(|t| matches!(t, Item::Blank)) {
        Some(idx) => trivia[..=idx].to_vec(),
        None => Vec::new(),
    }
}

// ============================================================================
// Rendering
// ============================================================================

/// Render items back into verbatim source lines.
fn render_items(items: &[Item]) -> Vec<String> {
    let mut out = Vec::new();
    for it in items {
        match it {
            Item::Blank => out.push(String::new()),
            Item::Comment(line) => out.push(line.clone()),
            Item::Entry(e) => {
                out.push(e.header.clone());
                out.extend(e.child_lines.iter().cloned());
            }
        }
    }
    out
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

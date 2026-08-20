//! Ordered CCL blocks (tables) with duplicate-key support.

use crate::item::Item;
use crate::repr::{Decor, Key, RawString};
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

/// A `key = value` pair inside a [`Table`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableEntry {
    pub(crate) key: Key,
    pub(crate) item: Item,
}

/// An ordered CCL block.
///
/// A table stores its entries in **source order** and allows duplicate keys,
/// because CCL gives both meaning: a repeated key is a list, and bare list
/// items (`= value`) are entries whose key is empty. Interleaving is preserved,
/// so `= a`, `x = 1`, `= b` renders back exactly as written.
///
/// Lookups are semantic: [`get`](Table::get) returns the first value for a key
/// and [`get_all`](Table::get_all) returns every value in order.
///
/// Unlike TOML there are no inline tables or arrays-of-tables — a CCL block is
/// always an indented run of entries, and nesting is expressed purely through
/// indentation.
#[derive(Debug, Clone, Default)]
pub struct Table {
    entries: Vec<TableEntry>,
    index: HashMap<String, Vec<usize>>,
    trailing: RawString,
}

impl Table {
    /// Create an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of entries, counting duplicate keys separately.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }

    /// Whether `key` occurs at least once.
    pub fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// The first item stored under `key`.
    pub fn get(&self, key: &str) -> Option<&Item> {
        self.position(key).map(|i| &self.entries[i].item)
    }

    /// Mutable access to the first item stored under `key`.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Item> {
        match self.position(key) {
            Some(i) => Some(&mut self.entries[i].item),
            None => None,
        }
    }

    /// Every item stored under `key`, in source order.
    pub fn get_all(&self, key: &str) -> Vec<&Item> {
        match self.index.get(key) {
            Some(positions) => positions.iter().map(|&i| &self.entries[i].item).collect(),
            None => Vec::new(),
        }
    }

    /// How many times `key` occurs.
    pub fn count(&self, key: &str) -> usize {
        self.index.get(key).map_or(0, Vec::len)
    }

    /// The CCL value of `key`, composing repeated occurrences.
    ///
    /// CCL is a monoid: repeating a key combines its values rather than
    /// replacing them. A key that occurs once yields its item unchanged, a key
    /// repeated with scalars yields a list, and a key repeated with blocks
    /// yields the merged block.
    ///
    /// ```
    /// use sickle::DocumentMut;
    ///
    /// let doc: DocumentMut = "user =\n  id = 1\nuser =\n  name = ada\n".parse().unwrap();
    /// let user = doc.as_table().get_composed("user").unwrap();
    /// assert_eq!(user.get_string(["id"]).unwrap(), "1");
    /// assert_eq!(user.get_string(["name"]).unwrap(), "ada");
    /// ```
    pub fn get_composed(&self, key: &str) -> Option<Item> {
        let values = self.get_all(key);
        match values.len() {
            0 => None,
            1 => Some(values[0].clone()),
            _ if values.iter().all(|item| item.is_value()) => {
                Some(Item::Array(values.into_iter().cloned().collect()))
            }
            _ => Some(
                values
                    .into_iter()
                    .fold(Item::None, |acc, item| compose(acc, item.clone())),
            ),
        }
    }

    /// The stored [`Key`] (including its formatting) for `key`.
    pub fn key(&self, key: &str) -> Option<&Key> {
        self.position(key).map(|i| &self.entries[i].key)
    }

    /// Mutable access to the stored [`Key`] for `key`.
    pub fn key_mut(&mut self, key: &str) -> Option<&mut Key> {
        match self.position(key) {
            Some(i) => Some(&mut self.entries[i].key),
            None => None,
        }
    }

    /// Insert `item` under `key`, replacing every existing occurrence.
    ///
    /// The new entry keeps the position of the first previous occurrence and
    /// its formatting, so replacing a value in a parsed document does not move
    /// or reindent it. Returns the previous first item, if any.
    pub fn insert(&mut self, key: impl Into<Key>, item: impl Into<Item>) -> Option<Item> {
        let key = key.into();
        let item = item.into();
        match self.position(key.get()) {
            Some(first) => {
                let previous = std::mem::replace(&mut self.entries[first].item, item);
                // Drop any later duplicates so the key now has a single value.
                let name = key.get().to_string();
                let mut seen_first = false;
                self.entries.retain(|entry| {
                    if entry.key.get() == name {
                        if seen_first {
                            return false;
                        }
                        seen_first = true;
                    }
                    true
                });
                if key.repr().is_set() {
                    self.entries[first].key = key;
                }
                self.rebuild_index();
                Some(previous)
            }
            None => {
                self.append(key, item);
                None
            }
        }
    }

    /// Append `item` under `key` without touching existing entries.
    ///
    /// This is how duplicate keys and bare list items (`Key::new("")`) are
    /// built programmatically.
    pub fn append(&mut self, key: impl Into<Key>, item: impl Into<Item>) {
        let key = key.into();
        let position = self.entries.len();
        self.index
            .entry(key.get().to_string())
            .or_default()
            .push(position);
        self.entries.push(TableEntry {
            key,
            item: item.into(),
        });
    }

    /// Remove every entry stored under `key`, returning the first item.
    ///
    /// Comments and blank lines written directly above a removed entry belong
    /// to it and are removed with it. The block's opening formatting is handed
    /// to whichever entry becomes first.
    pub fn remove(&mut self, key: &str) -> Option<Item> {
        if !self.index.contains_key(key) {
            return None;
        }
        let first_position = self.position(key);
        let detached = first_position.and_then(|p| self.detached_trivia_at(p));

        let mut removed: Option<Item> = None;
        let mut kept = Vec::with_capacity(self.entries.len());
        for entry in std::mem::take(&mut self.entries) {
            if entry.key.get() == key {
                if removed.is_none() {
                    removed = Some(entry.item);
                }
            } else {
                kept.push(entry);
            }
        }
        self.entries = kept;
        self.rebuild_index();
        if let Some(position) = first_position {
            self.adopt_trivia(position, detached);
        }
        removed
    }

    /// Remove the entry at `position` in source order.
    pub fn remove_at(&mut self, position: usize) -> Option<Item> {
        if position >= self.entries.len() {
            return None;
        }
        let detached = self.detached_trivia_at(position);
        let entry = self.entries.remove(position);
        self.rebuild_index();
        self.adopt_trivia(position, detached);
        Some(entry.item)
    }

    /// The part of an entry's leading trivia that outlives its removal.
    ///
    /// Everything up to and including the last blank line is a standalone block
    /// — a file header, a section separator — and survives. Anything after it
    /// was written for this entry specifically and goes away with it. When
    /// there is no blank line, only the newline that opens the block survives.
    fn detached_trivia_at(&self, position: usize) -> Option<String> {
        let prefix = self.entries.get(position)?.key.decor().prefix().as_str()?;
        Some(match prefix.rfind("\n\n") {
            Some(at) => prefix[..at + 2].to_string(),
            None if prefix.starts_with('\n') => "\n".to_string(),
            None => String::new(),
        })
    }

    /// Hand surviving trivia to the entry that took the removed entry's place.
    fn adopt_trivia(&mut self, position: usize, detached: Option<String>) {
        let Some(detached) = detached else { return };
        let Some(key) = self.entries.get_mut(position).map(|e| &mut e.key) else {
            return;
        };
        let current = key.decor().prefix().as_str().unwrap_or("").to_string();
        let rest = current.strip_prefix('\n').unwrap_or(&current);
        key.decor_mut().set_prefix(format!("{detached}{rest}"));
    }

    /// The indentation new entries in this block should use, inferred from the
    /// entries already present.
    pub(crate) fn inferred_indent(&self) -> Option<String> {
        self.entries.iter().find_map(|entry| {
            let prefix = entry.key.decor().prefix().as_str()?;
            Some(match prefix.rfind('\n') {
                Some(at) => prefix[at + 1..].to_string(),
                None => prefix.to_string(),
            })
        })
    }

    /// Whether entries in this block are separated from what precedes them by a
    /// newline, which every block except the document root is.
    pub(crate) fn opens_with_newline(&self) -> bool {
        self.entries.iter().any(|entry| {
            entry
                .key
                .decor()
                .prefix()
                .as_str()
                .is_some_and(|p| p.starts_with('\n'))
        })
    }

    /// Get an [`Entry`] for in-place manipulation of `key`.
    pub fn entry(&mut self, key: &str) -> Entry<'_> {
        self.entry_format(Key::new(key))
    }

    /// Like [`entry`](Table::entry) but lets the caller supply key formatting
    /// used when the key is vacant.
    pub fn entry_format(&mut self, key: Key) -> Entry<'_> {
        match self.position(key.get()) {
            Some(position) => Entry::Occupied(OccupiedEntry {
                table: self,
                position,
            }),
            None => Entry::Vacant(VacantEntry { table: self, key }),
        }
    }

    /// Iterate over every entry in source order, including duplicate keys.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Item)> {
        self.entries.iter().map(|e| (e.key.get(), &e.item))
    }

    /// Mutably iterate over every entry in source order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut Item)> {
        self.entries.iter_mut().map(|e| (e.key.get(), &mut e.item))
    }

    /// Iterate over every entry's [`Key`] and [`Item`] in source order.
    pub fn iter_keys(&self) -> impl Iterator<Item = (&Key, &Item)> {
        self.entries.iter().map(|e| (&e.key, &e.item))
    }

    /// Every key in source order, including duplicates.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.key.get())
    }

    /// Every distinct key, in order of first appearance.
    pub fn unique_keys(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .map(|e| e.key.get())
            .filter(|k| seen.insert(*k))
            .collect()
    }

    /// Every item in source order.
    pub fn values(&self) -> impl Iterator<Item = &Item> {
        self.entries.iter().map(|e| &e.item)
    }

    /// Mutable access to every item in source order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Item> {
        self.entries.iter_mut().map(|e| &mut e.item)
    }

    /// Whether every entry uses the empty (bare list item) key.
    ///
    /// A pure bare-list block is normalized to an [`Array`]
    /// during parsing; this predicate exists for tables assembled by hand.
    pub fn is_bare_list(&self) -> bool {
        !self.entries.is_empty() && self.entries.iter().all(|e| e.key.is_bare_list_item())
    }

    /// Trivia (comments and blank lines) following the last entry of the block.
    pub fn trailing(&self) -> &RawString {
        &self.trailing
    }

    /// Replace the trivia following the last entry of the block.
    pub fn set_trailing(&mut self, trailing: impl Into<RawString>) {
        self.trailing = trailing.into();
    }

    /// Canonicalize table syntax while retaining comments and blank lines.
    pub fn fmt(&mut self) {
        self.fmt_with_indent(0);
    }

    pub(crate) fn fmt_with_indent(&mut self, indent: usize) {
        self.trailing = canonical_trailing(&self.trailing, indent);
        for entry in &mut self.entries {
            entry.key.repr_mut().clear();
            let prefix = canonical_prefix(entry.key.decor().prefix(), indent);
            entry.key.decor_mut().clear();
            entry.key.decor_mut().set_prefix(prefix);
            fmt_item(&mut entry.item, indent + 2);
        }
    }

    pub(crate) fn position(&self, key: &str) -> Option<usize> {
        self.index.get(key).and_then(|v| v.first()).copied()
    }

    pub(crate) fn entry_key_mut(&mut self, position: usize) -> Option<&mut Key> {
        self.entries.get_mut(position).map(|e| &mut e.key)
    }

    pub(crate) fn entries(&self) -> &[TableEntry] {
        &self.entries
    }

    pub(crate) fn push_entry(&mut self, key: Key, item: Item) {
        self.append(key, item);
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (position, entry) in self.entries.iter().enumerate() {
            self.index
                .entry(entry.key.get().to_string())
                .or_default()
                .push(position);
        }
    }
}

impl PartialEq for Table {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl<K: Into<Key>, V: Into<Item>> FromIterator<(K, V)> for Table {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut table = Table::new();
        for (key, item) in iter {
            table.append(key, item);
        }
        table
    }
}

impl Index<&str> for Table {
    type Output = Item;

    /// Panics when `key` is absent; use [`get`](Table::get) for a checked read.
    fn index(&self, key: &str) -> &Item {
        self.get(key)
            .unwrap_or_else(|| panic!("key `{key}` not found in table"))
    }
}

impl IndexMut<&str> for Table {
    /// Auto-vivifies `key` with [`Item::None`] when absent, mirroring
    /// `toml_edit`. Use [`entry`](Table::entry) for explicit control.
    fn index_mut(&mut self, key: &str) -> &mut Item {
        if self.position(key).is_none() {
            self.append(Key::new(key), Item::None);
        }
        let position = self.position(key).expect("just inserted");
        &mut self.entries[position].item
    }
}

/// A view into a single table key, whether present or not.
pub enum Entry<'a> {
    /// The key already exists.
    Occupied(OccupiedEntry<'a>),
    /// The key does not exist yet.
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    /// The key this entry refers to.
    pub fn key(&self) -> &str {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }

    /// Return the existing item, inserting `default` when vacant.
    pub fn or_insert(self, default: impl Into<Item>) -> &'a mut Item {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// Return the existing item, inserting the result of `default` when vacant.
    pub fn or_insert_with<F, I>(self, default: F) -> &'a mut Item
    where
        F: FnOnce() -> I,
        I: Into<Item>,
    {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default()),
        }
    }
}

/// An occupied [`Entry`].
pub struct OccupiedEntry<'a> {
    table: &'a mut Table,
    position: usize,
}

impl<'a> OccupiedEntry<'a> {
    /// The key of this entry.
    pub fn key(&self) -> &str {
        self.table.entries[self.position].key.get()
    }

    /// A reference to the stored item.
    pub fn get(&self) -> &Item {
        &self.table.entries[self.position].item
    }

    /// A mutable reference to the stored item.
    pub fn get_mut(&mut self) -> &mut Item {
        &mut self.table.entries[self.position].item
    }

    /// Convert into a mutable reference with the table's lifetime.
    pub fn into_mut(self) -> &'a mut Item {
        &mut self.table.entries[self.position].item
    }

    /// Replace the stored item, returning the previous one.
    pub fn insert(&mut self, item: impl Into<Item>) -> Item {
        std::mem::replace(&mut self.table.entries[self.position].item, item.into())
    }

    /// Remove this entry from the table.
    pub fn remove(self) -> Item {
        let position = self.position;
        self.table
            .remove_at(position)
            .expect("occupied entries always exist")
    }
}

/// A vacant [`Entry`].
pub struct VacantEntry<'a> {
    table: &'a mut Table,
    key: Key,
}

impl<'a> VacantEntry<'a> {
    /// The key that would be inserted.
    pub fn key(&self) -> &str {
        self.key.get()
    }

    /// Insert `item` under this key and return a mutable reference to it.
    pub fn insert(self, item: impl Into<Item>) -> &'a mut Item {
        let position = self.table.entries.len();
        self.table.append(self.key, item);
        &mut self.table.entries[position].item
    }
}

/// One element of an [`Array`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArrayElement {
    pub(crate) decor: Decor,
    pub(crate) item: Item,
}

/// A CCL list.
///
/// Arrays model both CCL list spellings:
///
/// * a bare-list block, where every line is `= value`, and
/// * a repeated key, where the same key appears several times.
///
/// Every element keeps its own source representation, so a parsed list renders
/// back verbatim. Elements are [`Item`]s, not just scalars, because a CCL list
/// element may itself be a nested block.
#[derive(Debug, Clone, Default)]
pub struct Array {
    elements: Vec<ArrayElement>,
    kind: ArrayKind,
    trailing: RawString,
}

/// How a list is spelled in CCL source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayKind {
    /// `key =` followed by indented `= item` lines.
    #[default]
    BareList,
    /// The same key repeated once per element.
    RepeatedKey,
}

impl Array {
    /// Create an empty bare-syntax list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty list with the given source spelling.
    pub fn with_kind(kind: ArrayKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    /// How this list is spelled when rendered.
    pub fn kind(&self) -> ArrayKind {
        self.kind
    }

    /// Change how this list is spelled when rendered.
    pub fn set_kind(&mut self, kind: ArrayKind) {
        self.kind = kind;
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the list has no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The element at `index`.
    pub fn get(&self, index: usize) -> Option<&Item> {
        self.elements.get(index).map(|e| &e.item)
    }

    /// Mutable access to the element at `index`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Item> {
        self.elements.get_mut(index).map(|e| &mut e.item)
    }

    /// Append an element.
    pub fn push(&mut self, item: impl Into<Item>) {
        self.elements.push(ArrayElement {
            decor: Decor::default(),
            item: item.into(),
        });
    }

    /// Insert an element at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index > len()`.
    pub fn insert(&mut self, index: usize, item: impl Into<Item>) {
        self.elements.insert(
            index,
            ArrayElement {
                decor: Decor::default(),
                item: item.into(),
            },
        );
    }

    /// Remove and return the element at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index >= len()`.
    pub fn remove(&mut self, index: usize) -> Item {
        self.elements.remove(index).item
    }

    /// Remove every element.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Iterate over the elements.
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.elements.iter().map(|e| &e.item)
    }

    /// Mutably iterate over the elements.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Item> {
        self.elements.iter_mut().map(|e| &mut e.item)
    }

    /// Collect the elements as scalar text, or `None` when any element is a
    /// nested block.
    pub fn as_strings(&self) -> Option<Vec<&str>> {
        self.elements
            .iter()
            .map(|e| e.item.as_value().map(|v| v.as_str()))
            .collect()
    }

    /// Trivia following the last element of the list.
    pub fn trailing(&self) -> &RawString {
        &self.trailing
    }

    /// Replace the trivia following the last element of the list.
    pub fn set_trailing(&mut self, trailing: impl Into<RawString>) {
        self.trailing = trailing.into();
    }

    /// Canonicalize list syntax while retaining comments and blank lines.
    pub fn fmt(&mut self) {
        self.fmt_with_indent(0);
    }

    pub(crate) fn fmt_with_indent(&mut self, indent: usize) {
        self.trailing = canonical_trailing(&self.trailing, indent);
        for element in &mut self.elements {
            let prefix = canonical_prefix(element.decor.prefix(), indent);
            element.decor.clear();
            element.decor.set_prefix(prefix);
            fmt_item(&mut element.item, indent + 2);
        }
    }

    pub(crate) fn elements(&self) -> &[ArrayElement] {
        &self.elements
    }

    pub(crate) fn push_element(&mut self, decor: Decor, item: Item) {
        self.elements.push(ArrayElement { decor, item });
    }
}

fn fmt_item(item: &mut Item, indent: usize) {
    match item {
        Item::None => {}
        Item::Value(value) => value.fmt(),
        Item::Table(table) => table.fmt_with_indent(indent),
        Item::Array(array) => array.fmt_with_indent(indent),
    }
}

fn canonical_prefix(raw: &RawString, indent: usize) -> RawString {
    let Some(text) = raw.as_str() else {
        return RawString::unset();
    };

    let padding = " ".repeat(indent);
    let mut output = String::new();
    for part in text.split_inclusive('\n') {
        let has_newline = part.ends_with('\n');
        let line = part.strip_suffix('\n').unwrap_or(part);
        let line = line.strip_suffix('\r').unwrap_or(line);
        let trimmed = line.trim();

        if !trimmed.is_empty() {
            output.push_str(&padding);
            output.push_str(trimmed);
        } else if !has_newline {
            output.push_str(&padding);
        }

        if has_newline {
            output.push('\n');
        }
    }

    RawString::new(output)
}

fn canonical_trailing(raw: &RawString, indent: usize) -> RawString {
    let Some(text) = raw.as_str() else {
        return RawString::unset();
    };

    let contains_comment = text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("/=") || line.starts_with("/ =")
    });
    let contains_blank_line = text.matches('\n').count() > 1;

    if contains_comment || contains_blank_line {
        canonical_prefix(raw, indent)
    } else {
        RawString::unset()
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl<V: Into<Item>> FromIterator<V> for Array {
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        let mut array = Array::new();
        for item in iter {
            array.push(item);
        }
        array
    }
}

impl<V: Into<Item>> Extend<V> for Array {
    fn extend<I: IntoIterator<Item = V>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl Index<usize> for Array {
    type Output = Item;

    /// Panics when `index` is out of bounds; use [`get`](Array::get) instead.
    fn index(&self, index: usize) -> &Item {
        self.get(index)
            .unwrap_or_else(|| panic!("index {index} out of bounds in list"))
    }
}

impl IndexMut<usize> for Array {
    /// Panics when `index` is out of bounds; use [`get_mut`](Array::get_mut).
    fn index_mut(&mut self, index: usize) -> &mut Item {
        let len = self.len();
        self.get_mut(index)
            .unwrap_or_else(|| panic!("index {index} out of bounds in list of length {len}"))
    }
}

/// Combine two CCL values, following the language's monoid semantics.
///
/// Blocks merge key by key (recursively), scalars and lists concatenate into a
/// list, and an absent value is the identity.
///
/// ```
/// use sickle::{compose, table, value, Item, Table};
///
/// let mut left = Table::new();
/// left.append("a", value("1"));
/// let mut right = Table::new();
/// right.append("b", value("2"));
///
/// let merged = compose(Item::Table(left), Item::Table(right));
/// assert_eq!(merged.get_string(["a"]).unwrap(), "1");
/// assert_eq!(merged.get_string(["b"]).unwrap(), "2");
/// let _ = table();
/// ```
pub fn compose(left: Item, right: Item) -> Item {
    match (left, right) {
        (Item::None, other) | (other, Item::None) => other,
        (Item::Table(mut a), Item::Table(b)) => {
            for entry in b.entries() {
                let key = entry.key.get().to_string();
                match a.get(&key) {
                    Some(existing) => {
                        let merged = compose(existing.clone(), entry.item.clone());
                        a.insert(key, merged);
                    }
                    None => a.append(entry.key.clone(), entry.item.clone()),
                }
            }
            Item::Table(a)
        }
        (Item::Array(mut a), Item::Array(b)) => {
            for item in b.iter() {
                a.push(item.clone());
            }
            Item::Array(a)
        }
        (Item::Array(mut a), other) => {
            a.push(other);
            Item::Array(a)
        }
        (other, Item::Array(b)) => {
            let mut array = Array::new();
            array.push(other);
            for item in b.iter() {
                array.push(item.clone());
            }
            Item::Array(array)
        }
        (a, b) => {
            let mut array = Array::new();
            array.push(a);
            array.push(b);
            Item::Array(array)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{table, value};

    #[test]
    fn duplicate_keys_are_kept_in_order() {
        let mut t = Table::new();
        t.append("item", value("first"));
        t.append("other", value("x"));
        t.append("item", value("second"));

        assert_eq!(t.len(), 3);
        assert_eq!(t.count("item"), 2);
        assert_eq!(t.get("item").unwrap().as_str(), Some("first"));
        let all: Vec<_> = t
            .get_all("item")
            .into_iter()
            .map(|i| i.as_str().unwrap())
            .collect();
        assert_eq!(all, vec!["first", "second"]);
        assert_eq!(t.unique_keys(), vec!["item", "other"]);
    }

    #[test]
    fn insert_replaces_all_duplicates() {
        let mut t = Table::new();
        t.append("k", value("a"));
        t.append("z", value("z"));
        t.append("k", value("b"));

        let previous = t.insert("k", value("c")).unwrap();
        assert_eq!(previous.as_str(), Some("a"));
        assert_eq!(t.count("k"), 1);
        assert_eq!(t.len(), 2);
        assert_eq!(t.keys().collect::<Vec<_>>(), vec!["k", "z"]);
    }

    #[test]
    fn remove_drops_every_occurrence() {
        let mut t = Table::new();
        t.append("k", value("a"));
        t.append("k", value("b"));
        t.append("other", value("c"));

        let first = t.remove("k").unwrap();
        assert_eq!(first.as_str(), Some("a"));
        assert!(!t.contains_key("k"));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn entry_api_inserts_and_updates() {
        let mut t = Table::new();
        t.entry("server").or_insert(table());
        assert!(t.get("server").unwrap().is_table());

        match t.entry("server") {
            Entry::Occupied(mut e) => {
                e.insert(value("replaced"));
            }
            Entry::Vacant(_) => panic!("expected occupied"),
        }
        assert_eq!(t.get("server").unwrap().as_str(), Some("replaced"));
    }

    #[test]
    fn index_mut_autovivifies() {
        let mut t = Table::new();
        t["a"] = value("1");
        assert_eq!(t["a"].as_str(), Some("1"));
    }

    #[test]
    fn arrays_support_mixed_elements() {
        let mut a = Array::new();
        a.push(value("one"));
        a.push(table());
        assert_eq!(a.len(), 2);
        assert!(a.as_strings().is_none());
        assert_eq!(a[0].as_str(), Some("one"));

        a.remove(1);
        assert_eq!(a.as_strings(), Some(vec!["one"]));
    }

    #[test]
    fn bare_list_detection() {
        let mut t = Table::new();
        t.append("", value("a"));
        t.append("", value("b"));
        assert!(t.is_bare_list());
        t.append("x", value("c"));
        assert!(!t.is_bare_list());
    }
}

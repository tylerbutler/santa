//! Formatting metadata: raw source text, decorations, and keys.
//!
//! Every node in a Sickle document keeps the exact source text it came from so
//! an unmodified document renders back byte-for-byte. Nodes created
//! programmatically have no raw text and are rendered in canonical form
//! instead.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Verbatim source text attached to a node.
///
/// A `RawString` is either *set* (it holds the exact characters that appeared
/// in the input) or *unset* (the node was created programmatically and should
/// be rendered canonically).
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawString(Option<String>);

impl RawString {
    /// An unset raw string.
    pub const fn unset() -> Self {
        RawString(None)
    }

    /// Wrap explicit verbatim text.
    pub fn new(text: impl Into<String>) -> Self {
        RawString(Some(text.into()))
    }

    /// The verbatim text, if this raw string is set.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// The verbatim text, or `default` when unset.
    pub fn or<'a>(&'a self, default: &'a str) -> &'a str {
        self.0.as_deref().unwrap_or(default)
    }

    /// Whether this raw string carries verbatim text.
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Clear any verbatim text, forcing canonical rendering.
    pub fn clear(&mut self) {
        self.0 = None;
    }
}

impl From<&str> for RawString {
    fn from(value: &str) -> Self {
        RawString::new(value)
    }
}

impl From<String> for RawString {
    fn from(value: String) -> Self {
        RawString::new(value)
    }
}

impl fmt::Display for RawString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.or(""))
    }
}

/// Whitespace, blank lines, and comments surrounding a node.
///
/// For a key, `prefix` holds everything between the end of the previous entry
/// and the first character of the key — that is, any comment lines, blank
/// lines, and the entry's indentation — and `suffix` holds the whitespace
/// between the key and the `=` delimiter.
///
/// For a scalar value, `prefix` holds the whitespace between `=` and the first
/// character of the value, and `suffix` holds any trailing whitespace before
/// the newline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decor {
    prefix: RawString,
    suffix: RawString,
}

impl Decor {
    /// Create a decor with both halves set.
    pub fn new(prefix: impl Into<RawString>, suffix: impl Into<RawString>) -> Self {
        Self {
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }

    /// The leading raw text, if set.
    pub fn prefix(&self) -> &RawString {
        &self.prefix
    }

    /// The trailing raw text, if set.
    pub fn suffix(&self) -> &RawString {
        &self.suffix
    }

    /// Replace the leading raw text.
    pub fn set_prefix(&mut self, prefix: impl Into<RawString>) {
        self.prefix = prefix.into();
    }

    /// Replace the trailing raw text.
    pub fn set_suffix(&mut self, suffix: impl Into<RawString>) {
        self.suffix = suffix.into();
    }

    /// Drop both halves so the node renders canonically.
    pub fn clear(&mut self) {
        self.prefix.clear();
        self.suffix.clear();
    }
}

/// A table key together with its formatting.
///
/// Keys compare, hash, and order by their semantic text only; formatting is
/// ignored. The empty key is meaningful in CCL: it denotes a bare list item
/// (`= value`).
#[derive(Debug, Clone, Default, Eq)]
pub struct Key {
    key: String,
    repr: RawString,
    decor: Decor,
}

impl Key {
    /// Create a key with canonical formatting.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            repr: RawString::unset(),
            decor: Decor::default(),
        }
    }

    /// The semantic key text.
    pub fn get(&self) -> &str {
        &self.key
    }

    /// Consume the key, returning its semantic text.
    pub fn into_string(self) -> String {
        self.key
    }

    /// The verbatim key text from the source, if any.
    pub fn repr(&self) -> &RawString {
        &self.repr
    }

    /// Attach verbatim key text (builder form).
    pub fn with_repr(mut self, repr: impl Into<RawString>) -> Self {
        self.repr = repr.into();
        self
    }

    /// Attach surrounding formatting (builder form).
    pub fn with_decor(mut self, decor: Decor) -> Self {
        self.decor = decor;
        self
    }

    /// The formatting around this key.
    pub fn decor(&self) -> &Decor {
        &self.decor
    }

    /// Mutable access to the formatting around this key.
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }

    /// Replace the semantic key text, discarding any verbatim text.
    pub fn set(&mut self, key: impl Into<String>) {
        self.key = key.into();
        self.repr.clear();
    }

    /// Whether this is the empty key used by CCL bare list items.
    pub fn is_bare_list_item(&self) -> bool {
        self.key.is_empty()
    }

    pub(crate) fn repr_mut(&mut self) -> &mut RawString {
        &mut self.repr
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        &self.key
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::new(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::new(value)
    }
}

impl From<&String> for Key {
    fn from(value: &String) -> Self {
        Key::new(value.clone())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_string_set_and_unset() {
        let unset = RawString::unset();
        assert!(!unset.is_set());
        assert_eq!(unset.or("fallback"), "fallback");

        let set = RawString::new("  ");
        assert!(set.is_set());
        assert_eq!(set.or("fallback"), "  ");
    }

    #[test]
    fn keys_compare_by_text_only() {
        let plain = Key::new("name");
        let decorated = Key::new("name")
            .with_repr("  name")
            .with_decor(Decor::new("/= c\n", " "));
        assert_eq!(plain, decorated);
    }

    #[test]
    fn setting_key_clears_repr() {
        let mut key = Key::new("a").with_repr("  a");
        assert!(key.repr().is_set());
        key.set("b");
        assert_eq!(key.get(), "b");
        assert!(!key.repr().is_set());
    }

    #[test]
    fn empty_key_is_bare_list_item() {
        assert!(Key::new("").is_bare_list_item());
        assert!(!Key::new("x").is_bare_list_item());
    }
}

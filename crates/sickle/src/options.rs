//! Configurable CCL behaviors.
//!
//! CCL leaves a handful of behaviors up to the implementation. Sickle collects
//! all of them — both parse-time and access-time — into a single [`Options`]
//! value so a document, its typed reads, and its Serde views all agree.
//!
//! Defaults match the reference implementation: loose spacing, first-`=`
//! delimiter, tabs and CRLF preserved verbatim, strict booleans, and no list
//! coercion. See the [CCL behavior reference](https://ccl.tylerbutler.com/behavior-reference/).

use std::borrow::Cow;

/// How to handle spacing around the `=` delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpacingBehavior {
    /// Require spaces around `=` (`key = value`). `key=value` becomes a key
    /// with no value.
    Strict,
    /// Accept any whitespace, including none, around `=`. This is the default
    /// and matches the reference implementation.
    #[default]
    Loose,
}

/// Which `=` on a line separates the key from the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DelimiterStrategy {
    /// Always split on the first `=`. Keys cannot contain `=`.
    ///
    /// `a=b=c` parses as key `a`, value `b=c`.
    #[default]
    FirstEquals,
    /// Prefer ` = ` when present, falling back to the first `=`. This lets keys
    /// contain `=`, e.g. `https://x.com?q=1 = result`.
    PreferSpaced,
}

/// How to handle tab characters in parsed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabBehavior {
    /// Keep tabs verbatim in values (default).
    #[default]
    Preserve,
    /// Replace each tab with a single space.
    ToSpaces,
}

/// How to handle CRLF line endings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrlfBehavior {
    /// Keep `\r\n` verbatim (default).
    #[default]
    Preserve,
    /// Rewrite `\r\n` to `\n` before parsing.
    NormalizeToLf,
}

/// How strictly scalars are read as booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoolBehavior {
    /// Only `true`/`false` (case-insensitive). This is the default.
    #[default]
    Strict,
    /// Also accept `yes`/`no`, `on`/`off`, and `1`/`0` (case-insensitive).
    Lenient,
}

/// Whether single values can be read as one-element lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListBehavior {
    /// List reads only succeed on actual lists. This is the default.
    #[default]
    Strict,
    /// A single scalar is wrapped into a one-element list.
    Coerce,
}

/// Parse-time and access-time CCL behavior settings.
///
/// ```
/// use sickle::{DocumentMut, Options, BoolBehavior};
///
/// let doc = DocumentMut::parse_with("enabled = yes", &Options::new()).unwrap();
/// assert!(doc.get_bool(["enabled"]).is_err());
///
/// let lenient = Options::new().with_bool(BoolBehavior::Lenient);
/// let doc = DocumentMut::parse_with("enabled = yes", &lenient).unwrap();
/// assert!(doc.get_bool(["enabled"]).unwrap());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Spacing requirements around `=`.
    pub spacing: SpacingBehavior,
    /// Tab handling.
    pub tabs: TabBehavior,
    /// CRLF handling.
    pub crlf: CrlfBehavior,
    /// Delimiter selection strategy.
    pub delimiter: DelimiterStrategy,
    /// Boolean strictness for typed reads.
    pub boolean: BoolBehavior,
    /// List coercion for typed reads.
    pub list: ListBehavior,
}

impl Options {
    /// Options matching the reference implementation's defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Permissive options: loose spacing, tabs converted to spaces, CRLF
    /// normalized, spaced-delimiter preference, lenient booleans, and list
    /// coercion.
    pub fn permissive() -> Self {
        Self {
            spacing: SpacingBehavior::Loose,
            tabs: TabBehavior::ToSpaces,
            crlf: CrlfBehavior::NormalizeToLf,
            delimiter: DelimiterStrategy::PreferSpaced,
            boolean: BoolBehavior::Lenient,
            list: ListBehavior::Coerce,
        }
    }

    /// Set the spacing behavior.
    pub fn with_spacing(mut self, spacing: SpacingBehavior) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the tab handling behavior.
    pub fn with_tabs(mut self, tabs: TabBehavior) -> Self {
        self.tabs = tabs;
        self
    }

    /// Set the CRLF handling behavior.
    pub fn with_crlf(mut self, crlf: CrlfBehavior) -> Self {
        self.crlf = crlf;
        self
    }

    /// Set the delimiter strategy.
    pub fn with_delimiter(mut self, delimiter: DelimiterStrategy) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Set the boolean strictness used by typed reads.
    pub fn with_bool(mut self, boolean: BoolBehavior) -> Self {
        self.boolean = boolean;
        self
    }

    /// Set the list coercion behavior used by typed reads.
    pub fn with_list(mut self, list: ListBehavior) -> Self {
        self.list = list;
        self
    }

    /// Whether spacing around `=` is strict.
    pub fn is_strict_spacing(&self) -> bool {
        matches!(self.spacing, SpacingBehavior::Strict)
    }

    /// Whether the spaced ` = ` delimiter is preferred.
    pub fn prefers_spaced_delimiter(&self) -> bool {
        matches!(self.delimiter, DelimiterStrategy::PreferSpaced)
    }

    /// Whether tabs are preserved verbatim.
    pub fn preserves_tabs(&self) -> bool {
        matches!(self.tabs, TabBehavior::Preserve)
    }

    /// Whether CRLF sequences are preserved verbatim.
    pub fn preserves_crlf(&self) -> bool {
        matches!(self.crlf, CrlfBehavior::Preserve)
    }

    /// Whether booleans are read leniently.
    pub fn is_lenient_bool(&self) -> bool {
        matches!(self.boolean, BoolBehavior::Lenient)
    }

    /// Whether single values coerce into one-element lists.
    pub fn coerces_lists(&self) -> bool {
        matches!(self.list, ListBehavior::Coerce)
    }

    pub(crate) fn process_tabs<'a>(&self, s: &'a str) -> Cow<'a, str> {
        if self.preserves_tabs() {
            Cow::Borrowed(s)
        } else {
            Cow::Owned(s.replace('\t', " "))
        }
    }

    pub(crate) fn process_crlf<'a>(&self, s: &'a str) -> Cow<'a, str> {
        if self.preserves_crlf() {
            Cow::Borrowed(s)
        } else {
            Cow::Owned(s.replace("\r\n", "\n"))
        }
    }

    /// Parse `text` as a boolean using the configured strictness.
    pub(crate) fn parse_bool(&self, text: &str) -> Option<bool> {
        let lower = text.to_lowercase();
        if self.is_lenient_bool() {
            match lower.as_str() {
                "true" | "yes" | "on" | "1" => Some(true),
                "false" | "no" | "off" | "0" => Some(false),
                _ => None,
            }
        } else {
            match lower.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_reference_behavior() {
        let opts = Options::new();
        assert!(!opts.is_strict_spacing());
        assert!(opts.preserves_tabs());
        assert!(opts.preserves_crlf());
        assert!(!opts.prefers_spaced_delimiter());
        assert!(!opts.is_lenient_bool());
        assert!(!opts.coerces_lists());
    }

    #[test]
    fn permissive_flips_every_behavior() {
        let opts = Options::permissive();
        assert!(!opts.preserves_tabs());
        assert!(!opts.preserves_crlf());
        assert!(opts.prefers_spaced_delimiter());
        assert!(opts.is_lenient_bool());
        assert!(opts.coerces_lists());
    }

    #[test]
    fn builder_methods_are_independent() {
        let opts = Options::new()
            .with_tabs(TabBehavior::ToSpaces)
            .with_bool(BoolBehavior::Lenient);
        assert!(!opts.preserves_tabs());
        assert!(opts.is_lenient_bool());
        assert!(opts.preserves_crlf());
    }

    #[test]
    fn tab_and_crlf_processing() {
        let to_spaces = Options::new().with_tabs(TabBehavior::ToSpaces);
        assert_eq!(to_spaces.process_tabs("a\tb"), "a b");
        assert_eq!(Options::new().process_tabs("a\tb"), "a\tb");

        let normalize = Options::new().with_crlf(CrlfBehavior::NormalizeToLf);
        assert_eq!(normalize.process_crlf("a\r\nb"), "a\nb");
        assert_eq!(Options::new().process_crlf("a\r\nb"), "a\r\nb");
    }

    #[test]
    fn bool_parsing_respects_strictness() {
        let strict = Options::new();
        assert_eq!(strict.parse_bool("TRUE"), Some(true));
        assert_eq!(strict.parse_bool("yes"), None);

        let lenient = Options::new().with_bool(BoolBehavior::Lenient);
        assert_eq!(lenient.parse_bool("yes"), Some(true));
        assert_eq!(lenient.parse_bool("off"), Some(false));
        assert_eq!(lenient.parse_bool("maybe"), None);
    }
}

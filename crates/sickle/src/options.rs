//! Parse-time configuration options for CCL parsing behavior
//!
//! This module provides configurable behaviors that affect how CCL text is parsed.
//! These options are applied at parse time when calling `parse_with_options()` or
//! `load_with_options()`. All options have sensible defaults that match the reference
//! implementation.
//!
//! Note: Options that only affect specific APIs (like boolean parsing for `get_bool()`)
//! are not included here - they belong with those APIs. See `ListOptions` for access-time
//! configuration.

/// How to handle spacing around the `=` delimiter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpacingBehavior {
    /// Strict spacing: requires spaces around `=` (e.g., `key = value`)
    /// This is the default and matches the reference implementation.
    #[default]
    Strict,
    /// Loose spacing: allows any whitespace (including tabs) or no whitespace
    /// around `=` (e.g., `key=value`, `key  =  value`, `key\t=\tvalue`)
    Loose,
}

/// How to handle tab characters in parsed content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabBehavior {
    /// Preserve tabs as-is in values (default)
    #[default]
    Preserve,
    /// Convert tabs to spaces (single space per tab)
    ToSpaces,
}

/// How to handle CRLF line endings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrlfBehavior {
    /// Preserve CRLF line endings in values (default)
    #[default]
    Preserve,
    /// Normalize CRLF to LF
    NormalizeToLf,
}

/// Parse-time configuration options for CCL parsing
///
/// Controls parsing behaviors that can differ between implementations.
/// All options default to the reference implementation's behavior.
///
/// These options affect how the raw CCL text is tokenized and parsed.
/// They are applied once at parse time and cannot be changed after parsing.
/// Options that only affect specific accessor methods (like `get_bool()`)
/// are configured separately on those methods (see `ListOptions` for access-time options).
#[derive(Debug, Clone, Default)]
pub struct ParserOptions {
    /// How to handle spacing around `=`
    pub spacing: SpacingBehavior,
    /// How to handle tab characters
    pub tabs: TabBehavior,
    /// How to handle CRLF line endings
    pub crlf: CrlfBehavior,
}

impl ParserOptions {
    /// Create new parser options with default (strict/reference-compliant) settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create parser options with loose/permissive settings
    ///
    /// This enables:
    /// - Loose spacing (accepts `key=value`, `key = value`, etc.)
    /// - Tab-to-spaces conversion
    /// - CRLF normalization to LF
    pub fn permissive() -> Self {
        Self {
            spacing: SpacingBehavior::Loose,
            tabs: TabBehavior::ToSpaces,
            crlf: CrlfBehavior::NormalizeToLf,
        }
    }

    /// Set the spacing behavior
    pub fn with_spacing(mut self, spacing: SpacingBehavior) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the tab handling behavior
    pub fn with_tabs(mut self, tabs: TabBehavior) -> Self {
        self.tabs = tabs;
        self
    }

    /// Set the CRLF handling behavior
    pub fn with_crlf(mut self, crlf: CrlfBehavior) -> Self {
        self.crlf = crlf;
        self
    }

    /// Check if spacing is strict
    pub(crate) fn is_strict_spacing(&self) -> bool {
        matches!(self.spacing, SpacingBehavior::Strict)
    }

    /// Check if tabs should be preserved
    pub(crate) fn preserve_tabs(&self) -> bool {
        matches!(self.tabs, TabBehavior::Preserve)
    }

    /// Check if CRLF should be preserved
    pub(crate) fn preserve_crlf(&self) -> bool {
        matches!(self.crlf, CrlfBehavior::Preserve)
    }

    /// Process tabs in a string based on the configured tab behavior
    ///
    /// - `Preserve`: returns the string unchanged
    /// - `ToSpaces`: replaces each tab with a single space
    pub(crate) fn process_tabs<'a>(&self, s: &'a str) -> std::borrow::Cow<'a, str> {
        if self.preserve_tabs() {
            std::borrow::Cow::Borrowed(s)
        } else {
            std::borrow::Cow::Owned(s.replace('\t', " "))
        }
    }

    /// Process CRLF line endings based on the configured CRLF behavior
    ///
    /// - `Preserve`: returns the string unchanged
    /// - `NormalizeToLf`: replaces CRLF with LF
    pub(crate) fn process_crlf<'a>(&self, s: &'a str) -> std::borrow::Cow<'a, str> {
        if self.preserve_crlf() {
            std::borrow::Cow::Borrowed(s)
        } else {
            std::borrow::Cow::Owned(s.replace("\r\n", "\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = ParserOptions::new();
        assert!(opts.is_strict_spacing());
        assert!(opts.preserve_tabs());
        assert!(opts.preserve_crlf());
    }

    #[test]
    fn test_permissive_options() {
        let opts = ParserOptions::permissive();
        assert!(!opts.is_strict_spacing());
        assert!(!opts.preserve_tabs());
        assert!(!opts.preserve_crlf());
    }

    #[test]
    fn test_builder_pattern() {
        let opts = ParserOptions::new()
            .with_spacing(SpacingBehavior::Loose)
            .with_tabs(TabBehavior::ToSpaces);

        assert!(!opts.is_strict_spacing());
        assert!(!opts.preserve_tabs());
        // Others remain default
        assert!(opts.preserve_crlf());
    }

    #[test]
    fn test_process_tabs_preserve() {
        let opts = ParserOptions::new(); // Default preserves tabs
        let input = "hello\tworld";
        let result = opts.process_tabs(input);
        assert_eq!(result, "hello\tworld");
    }

    #[test]
    fn test_process_tabs_to_spaces() {
        let opts = ParserOptions::new().with_tabs(TabBehavior::ToSpaces);
        let input = "hello\tworld";
        let result = opts.process_tabs(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_process_tabs_multiple() {
        let opts = ParserOptions::new().with_tabs(TabBehavior::ToSpaces);
        let input = "\t\tindented\ttext\t";
        let result = opts.process_tabs(input);
        assert_eq!(result, "  indented text ");
    }

    #[test]
    fn test_process_crlf_preserve() {
        let opts = ParserOptions::new(); // Default preserves CRLF
        let input = "line1\r\nline2";
        let result = opts.process_crlf(input);
        assert_eq!(result, "line1\r\nline2");
    }

    #[test]
    fn test_process_crlf_normalize() {
        let opts = ParserOptions::new().with_crlf(CrlfBehavior::NormalizeToLf);
        let input = "line1\r\nline2\r\nline3";
        let result = opts.process_crlf(input);
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_process_crlf_mixed_endings() {
        let opts = ParserOptions::new().with_crlf(CrlfBehavior::NormalizeToLf);
        let input = "line1\r\nline2\nline3\r\n";
        let result = opts.process_crlf(input);
        assert_eq!(result, "line1\nline2\nline3\n");
    }
}

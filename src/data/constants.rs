use std::sync::LazyLock;
use string_interner::{DefaultSymbol, StringInterner};

pub static BUILTIN_SOURCES: &str = include_str!("../../data/sources.yaml");
pub static BUILTIN_PACKAGES: &str = include_str!("../../data/known_packages.yaml");
pub static DEFAULT_CONFIG: &str = include_str!("../../data/santa-config.yaml");

/// Global string interner for commonly used strings like package manager names
/// This reduces memory usage when the same strings are used repeatedly
pub static STRING_INTERNER: LazyLock<std::sync::RwLock<StringInterner>> =
    LazyLock::new(|| std::sync::RwLock::new(StringInterner::default()));

/// Type alias for interned string symbols
pub type InternedString = DefaultSymbol;

/// Helper function to intern a string
pub fn intern_string(s: &str) -> InternedString {
    STRING_INTERNER.write().unwrap().get_or_intern(s)
}

/// Helper function to resolve an interned string
pub fn resolve_string(symbol: InternedString) -> Option<String> {
    STRING_INTERNER
        .read()
        .unwrap()
        .resolve(symbol)
        .map(|s| s.to_string())
}

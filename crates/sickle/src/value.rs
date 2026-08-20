//! CCL scalar values.

use crate::options::Options;
use crate::repr::{Decor, RawString};
use std::fmt;

/// A CCL scalar.
///
/// CCL has exactly one scalar kind: text. Numbers and booleans are lexical
/// conventions layered on top of it, so a `Value` always keeps its text and
/// offers checked conversions ([`as_integer`](Value::as_integer),
/// [`as_float`](Value::as_float), [`as_bool`](Value::as_bool)) rather than
/// separate typed variants. This is a deliberate difference from TOML-shaped
/// APIs: there are no date-times, no inline tables, and no numeric literal
/// types to preserve.
///
/// A value parsed from source also keeps its verbatim text, which may differ
/// from the semantic text (for example trailing whitespace, or tabs when
/// [`TabBehavior::ToSpaces`](crate::TabBehavior::ToSpaces) is active).
#[derive(Debug, Clone, Default, Eq)]
pub struct Value {
    value: String,
    repr: RawString,
    decor: Decor,
}

impl Value {
    /// Create a scalar with canonical formatting.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            repr: RawString::unset(),
            decor: Decor::default(),
        }
    }

    pub(crate) fn from_parts(value: String, repr: RawString, decor: Decor) -> Self {
        Self { value, repr, decor }
    }

    /// The semantic text of this scalar.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Consume the scalar, returning its semantic text.
    pub fn into_string(self) -> String {
        self.value
    }

    /// Replace the semantic text, discarding the verbatim source text.
    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.repr.clear();
    }

    /// The verbatim source text, if this scalar came from parsing.
    pub fn repr(&self) -> &RawString {
        &self.repr
    }

    /// Attach verbatim source text (builder form).
    pub fn with_repr(mut self, repr: impl Into<RawString>) -> Self {
        self.repr = repr.into();
        self
    }

    /// Attach surrounding formatting (builder form).
    pub fn with_decor(mut self, decor: Decor) -> Self {
        self.decor = decor;
        self
    }

    /// The formatting around this scalar.
    pub fn decor(&self) -> &Decor {
        &self.decor
    }

    /// Mutable access to the formatting around this scalar.
    pub fn decor_mut(&mut self) -> &mut Decor {
        &mut self.decor
    }

    /// Whether the scalar text is empty (`key =`).
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Read the scalar as an `i64`.
    pub fn as_integer(&self) -> Option<i64> {
        self.value.trim().parse::<i64>().ok()
    }

    /// Read the scalar as an `f64`.
    pub fn as_float(&self) -> Option<f64> {
        self.value.trim().parse::<f64>().ok()
    }

    /// Read the scalar as a boolean using strict CCL rules (`true`/`false`,
    /// case-insensitive).
    pub fn as_bool(&self) -> Option<bool> {
        self.as_bool_with(&Options::new())
    }

    /// Read the scalar as a boolean using the supplied behavior options.
    pub fn as_bool_with(&self, options: &Options) -> Option<bool> {
        options.parse_bool(self.value.trim())
    }

    /// Whether the scalar text parses as an integer.
    pub fn is_integer(&self) -> bool {
        self.as_integer().is_some()
    }

    /// Whether the scalar text parses as a float.
    pub fn is_float(&self) -> bool {
        self.as_float().is_some()
    }

    /// Whether the scalar text parses as a strict boolean.
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Drop verbatim text and decorations so this scalar renders canonically.
    pub fn fmt(&mut self) {
        self.repr.clear();
        self.decor.clear();
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::new(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::new(value)
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Value::new(value.clone())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::new(value.to_string())
    }
}

macro_rules! value_from_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Value::new(value.to_string())
                }
            }
        )*
    };
}

value_from_number!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::BoolBehavior;

    #[test]
    fn scalar_keeps_text_and_converts() {
        let v = Value::new("42");
        assert_eq!(v.as_str(), "42");
        assert_eq!(v.as_integer(), Some(42));
        assert_eq!(v.as_float(), Some(42.0));
        assert_eq!(v.as_bool(), None);
    }

    #[test]
    fn strict_bools_reject_yes() {
        let v = Value::new("yes");
        assert_eq!(v.as_bool(), None);
        let lenient = Options::new().with_bool(BoolBehavior::Lenient);
        assert_eq!(v.as_bool_with(&lenient), Some(true));
    }

    #[test]
    fn setting_text_clears_repr() {
        let mut v = Value::new("a").with_repr("a  ");
        assert!(v.repr().is_set());
        v.set("b");
        assert!(!v.repr().is_set());
        assert_eq!(v.as_str(), "b");
    }

    #[test]
    fn numbers_and_bools_convert_into_values() {
        assert_eq!(Value::from(7i64).as_str(), "7");
        assert_eq!(Value::from(true).as_str(), "true");
        assert_eq!(Value::from(1.5f64).as_str(), "1.5");
    }

    #[test]
    fn values_compare_by_text() {
        let a = Value::new("x");
        let b = Value::new("x").with_repr(" x ");
        assert_eq!(a, b);
    }
}

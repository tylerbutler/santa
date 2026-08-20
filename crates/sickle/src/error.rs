//! Structured error types for parsing, reading, and editing CCL documents.
//!
//! Sickle separates failures into three families so callers can react to each
//! precisely:
//!
//! * [`ParseError`] — the input is not well-formed CCL. Carries a [`Position`]
//!   with a byte offset plus 1-based line/column for diagnostics.
//! * [`GetError`] — a checked path lookup failed: the path was empty, a key or
//!   index was missing, or the node had a different shape than requested.
//! * [`EditError`] — a checked mutation could not be applied: the path was
//!   empty, an intermediate node was not a table/array, or a comment was
//!   unrepresentable.
//!
//! [`Error`] unifies all of them (plus the optional Serde errors) for callers
//! that just want one error type.

use std::fmt;

/// A location in the source text.
///
/// `offset` is a byte offset from the start of the input; `line` and `column`
/// are 1-based and counted in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Byte offset from the start of the input.
    pub offset: usize,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number, counted in characters.
    pub column: usize,
}

impl Position {
    /// Create a position from an explicit offset and line/column pair.
    pub fn new(offset: usize, line: usize, column: usize) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }

    /// Compute the position of `offset` within `input`.
    ///
    /// Offsets past the end of `input` are clamped to the end.
    pub fn from_offset(input: &str, offset: usize) -> Self {
        let offset = offset.min(input.len());
        let mut line = 1usize;
        let mut column = 1usize;
        for (idx, ch) in input.char_indices() {
            if idx >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        Self {
            offset,
            line,
            column,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// The input could not be parsed as CCL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    position: Position,
}

impl ParseError {
    /// Create a parse error at an already-computed position.
    pub fn new(message: impl Into<String>, position: Position) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }

    /// Create a parse error, computing line/column from `input` and `offset`.
    pub fn at_offset(message: impl Into<String>, input: &str, offset: usize) -> Self {
        Self::new(message, Position::from_offset(input, offset))
    }

    /// The human-readable description of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the failure occurred.
    pub fn position(&self) -> Position {
        self.position
    }

    /// The 1-based line and column of the failure.
    pub fn line_column(&self) -> (usize, usize) {
        (self.position.line, self.position.column)
    }

    /// The byte offset of the failure.
    pub fn offset(&self) -> usize {
        self.position.offset
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

/// The kind of node or scalar a checked accessor required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpectedType {
    /// Any present node.
    Value,
    /// A scalar.
    String,
    /// A scalar parseable as `i64`.
    Integer,
    /// A scalar parseable as `f64`.
    Float,
    /// A scalar parseable as a boolean.
    Boolean,
    /// A list (bare-list block, repeated key, or constructed array).
    List,
    /// A nested block.
    Table,
    /// Nothing (an absent node).
    None,
}

impl ExpectedType {
    /// The name used in error messages.
    pub fn name(&self) -> &'static str {
        match self {
            ExpectedType::Value => "value",
            ExpectedType::String => "string",
            ExpectedType::Integer => "integer",
            ExpectedType::Float => "float",
            ExpectedType::Boolean => "boolean",
            ExpectedType::List => "list",
            ExpectedType::Table => "table",
            ExpectedType::None => "none",
        }
    }
}

impl fmt::Display for ExpectedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A checked read through a path failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GetError {
    /// The path had no segments.
    EmptyPath,
    /// A key segment did not exist.
    MissingKey {
        /// The full path that was requested.
        path: String,
        /// The key that was missing.
        key: String,
    },
    /// An index segment was out of range.
    IndexOutOfBounds {
        /// The full path that was requested.
        path: String,
        /// The index that was requested.
        index: usize,
        /// The number of elements available.
        len: usize,
    },
    /// The node had a different shape than requested.
    TypeMismatch {
        /// The full path that was requested.
        path: String,
        /// The type the caller asked for.
        expected: ExpectedType,
        /// The type actually stored.
        found: ExpectedType,
    },
    /// The node was a scalar but its text could not be read as the requested type.
    InvalidValue {
        /// The full path that was requested.
        path: String,
        /// The type the caller asked for.
        expected: ExpectedType,
        /// The scalar text that failed to convert.
        value: String,
    },
}

impl fmt::Display for GetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetError::EmptyPath => f.write_str("path must have at least one segment"),
            GetError::MissingKey { path, key } => {
                write!(f, "key `{key}` not found at path `{path}`")
            }
            GetError::IndexOutOfBounds { path, index, len } => write!(
                f,
                "index {index} out of bounds at path `{path}` (length {len})"
            ),
            GetError::TypeMismatch {
                path,
                expected,
                found,
            } => write!(f, "expected {expected} at path `{path}`, found {found}"),
            GetError::InvalidValue {
                path,
                expected,
                value,
            } => write!(
                f,
                "value `{value}` at path `{path}` is not a valid {expected}"
            ),
        }
    }
}

impl std::error::Error for GetError {}

/// A checked edit through a path failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditError {
    /// The path had no segments.
    EmptyPath,
    /// An intermediate node existed but was not a table, so it cannot be descended into.
    NotATable {
        /// The path prefix that was not a table.
        path: String,
    },
    /// An index segment was used on a node that is not an array.
    NotAnArray {
        /// The path prefix that was not an array.
        path: String,
    },
    /// An index segment was out of range.
    IndexOutOfBounds {
        /// The full path that was requested.
        path: String,
        /// The index that was requested.
        index: usize,
        /// The number of elements available.
        len: usize,
    },
    /// The key targeted for removal did not exist.
    MissingKey {
        /// The full path that was requested.
        path: String,
        /// The key that was missing.
        key: String,
    },
    /// The comment text cannot be represented in CCL.
    InvalidComment {
        /// Why the comment was rejected.
        reason: String,
    },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::EmptyPath => f.write_str("path must have at least one segment"),
            EditError::NotATable { path } => {
                write!(f, "cannot descend into `{path}`: not a table")
            }
            EditError::NotAnArray { path } => {
                write!(f, "cannot index into `{path}`: not a list")
            }
            EditError::IndexOutOfBounds { path, index, len } => write!(
                f,
                "index {index} out of bounds at path `{path}` (length {len})"
            ),
            EditError::MissingKey { path, key } => {
                write!(f, "key `{key}` not found at path `{path}`")
            }
            EditError::InvalidComment { reason } => write!(f, "invalid comment: {reason}"),
        }
    }
}

impl std::error::Error for EditError {}

/// Serde deserialization failed.
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeserializeError {
    message: String,
}

#[cfg(feature = "serde")]
impl DeserializeError {
    /// Create a deserialization error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The human-readable description of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(feature = "serde")]
impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(feature = "serde")]
impl std::error::Error for DeserializeError {}

#[cfg(feature = "serde")]
impl serde::de::Error for DeserializeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new(msg.to_string())
    }
}

/// Serde serialization failed.
#[cfg(feature = "serde")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializeError {
    message: String,
}

#[cfg(feature = "serde")]
impl SerializeError {
    /// Create a serialization error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The human-readable description of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(feature = "serde")]
impl fmt::Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(feature = "serde")]
impl std::error::Error for SerializeError {}

#[cfg(feature = "serde")]
impl serde::ser::Error for SerializeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new(msg.to_string())
    }
}

/// The unified error type returned by Sickle's top-level helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input was not well-formed CCL.
    Parse(ParseError),
    /// A checked read failed.
    Get(GetError),
    /// A checked edit failed.
    Edit(EditError),
    /// Serde deserialization failed.
    #[cfg(feature = "serde")]
    Deserialize(DeserializeError),
    /// Serde serialization failed.
    #[cfg(feature = "serde")]
    Serialize(SerializeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => write!(f, "parse error: {e}"),
            Error::Get(e) => write!(f, "get error: {e}"),
            Error::Edit(e) => write!(f, "edit error: {e}"),
            #[cfg(feature = "serde")]
            Error::Deserialize(e) => write!(f, "deserialize error: {e}"),
            #[cfg(feature = "serde")]
            Error::Serialize(e) => write!(f, "serialize error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Parse(e) => Some(e),
            Error::Get(e) => Some(e),
            Error::Edit(e) => Some(e),
            #[cfg(feature = "serde")]
            Error::Deserialize(e) => Some(e),
            #[cfg(feature = "serde")]
            Error::Serialize(e) => Some(e),
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

impl From<GetError> for Error {
    fn from(e: GetError) -> Self {
        Error::Get(e)
    }
}

impl From<EditError> for Error {
    fn from(e: EditError) -> Self {
        Error::Edit(e)
    }
}

#[cfg(feature = "serde")]
impl From<DeserializeError> for Error {
    fn from(e: DeserializeError) -> Self {
        Error::Deserialize(e)
    }
}

#[cfg(feature = "serde")]
impl From<SerializeError> for Error {
    fn from(e: SerializeError) -> Self {
        Error::Serialize(e)
    }
}

/// Convenience alias for results carrying the unified [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_from_offset_tracks_lines_and_columns() {
        let input = "a = 1\nbb = 2\n";
        assert_eq!(Position::from_offset(input, 0), Position::new(0, 1, 1));
        assert_eq!(Position::from_offset(input, 6), Position::new(6, 2, 1));
        assert_eq!(Position::from_offset(input, 8), Position::new(8, 2, 3));
    }

    #[test]
    fn position_from_offset_clamps_past_end() {
        let input = "a = 1";
        let pos = Position::from_offset(input, 500);
        assert_eq!(pos.offset, input.len());
        assert_eq!(pos.line, 1);
    }

    #[test]
    fn parse_error_reports_line_column() {
        let err = ParseError::at_offset("boom", "a = 1\nb = 2", 6);
        assert_eq!(err.line_column(), (2, 1));
        assert_eq!(err.offset(), 6);
        assert!(err.to_string().contains("line 2, column 1"));
    }

    #[test]
    fn get_error_messages_include_path() {
        let err = GetError::MissingKey {
            path: "server.host".into(),
            key: "host".into(),
        };
        assert!(err.to_string().contains("server.host"));
    }

    #[test]
    fn unified_error_wraps_families() {
        let err: Error = GetError::EmptyPath.into();
        assert!(matches!(err, Error::Get(_)));
        assert!(err.to_string().starts_with("get error"));
    }
}

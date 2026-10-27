//! Values, positions and provenance.

use std::fmt;

use crate::source::Tier;

/// Resolved configuration value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// Explicit absence (`null`).
    Null,
    /// Boolean.
    Bool(bool),
    /// 64-bit integer.
    Int(i64),
    /// Floating point number.
    Float(f64),
    /// Text.
    Str(String),
}

impl Value {
    /// Returns the underlying string when the value is textual.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    /// `true` when the value is textual.
    #[must_use]
    pub fn is_str(&self) -> bool {
        matches!(self, Self::Str(_))
    }

    /// Builds a textual value.
    #[must_use]
    pub fn str(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(x) => write!(f, "{x}"),
            Self::Str(s) => f.write_str(s),
        }
    }
}

/// Position of a value inside a document.
///
/// The column is always exposed in **characters**, not bytes: the TOML and JSON parsers
/// work with byte offsets and they are normalised when the span is built.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// File name as declared by the source.
    pub file: String,
    /// Line, 1-based.
    pub line: u32,
    /// Column, 0-based, in characters.
    pub col: u32,
}

/// Where a value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Logical source name (`file:app.yaml`, `env`).
    pub source: String,
    /// Precedence band.
    pub tier: Tier,
    /// Application order inside the band.
    pub order: u32,
    /// Position in the document, when the source knows it.
    pub span: Option<Span>,
}

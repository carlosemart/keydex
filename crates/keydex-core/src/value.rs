//! Values, nodes, positions and provenance.

use std::collections::BTreeMap;
use std::fmt;

use crate::source::Tier;

/// Configuration value.
///
/// Maps and sequences hold [`Node`] children, so the tree carries provenance at every
/// level and can express structural replacement.
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
    /// Ordered sequence. A higher-priority source replaces it whole.
    Seq(Vec<Node>),
    /// String-keyed map. Higher-priority sources merge into it key by key.
    Map(BTreeMap<String, Node>),
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

    /// `true` when the value is a map or a sequence.
    #[must_use]
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Seq(_) | Self::Map(_))
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
            Self::Seq(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", item.value)?;
                }
                f.write_str("]")
            }
            Self::Map(entries) => {
                f.write_str("{")?;
                for (i, (key, item)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {}", item.value)?;
                }
                f.write_str("}")
            }
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

/// A configuration node: a value plus where it came from.
///
/// `provenance` is `None` only in nodes freshly produced by a parser, before the merge.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The value itself, with children for maps and sequences.
    pub value: Value,
    /// Position in the document, when the parser knows it.
    pub span: Option<Span>,
    /// Source that defined this node.
    pub provenance: Option<Provenance>,
    /// Sources that defined this path and lost, oldest first.
    pub shadowed: Vec<Provenance>,
    /// Keys referenced through interpolation.
    pub refs: Vec<String>,
}

impl Default for Node {
    fn default() -> Self {
        Self::empty_map()
    }
}

impl Node {
    /// Builds a node without provenance (parser output).
    #[must_use]
    pub fn new(value: Value, span: Option<Span>) -> Self {
        Self {
            value,
            span,
            provenance: None,
            shadowed: Vec::new(),
            refs: Vec::new(),
        }
    }

    /// Builds an empty map node.
    #[must_use]
    pub fn empty_map() -> Self {
        Self::new(Value::Map(BTreeMap::new()), None)
    }

    /// Attaches provenance, keeping the existing span unless the node has none.
    #[must_use]
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        if self.span.is_none() {
            self.span = provenance.span.clone();
        }
        self.provenance = Some(provenance);
        self
    }

    /// Returns the textual value, when the node is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_str()
    }

    /// Effective span: the node's own or the provenance's.
    #[must_use]
    pub fn effective_span(&self) -> Option<&Span> {
        self.span
            .as_ref()
            .or_else(|| self.provenance.as_ref().and_then(|p| p.span.as_ref()))
    }
}

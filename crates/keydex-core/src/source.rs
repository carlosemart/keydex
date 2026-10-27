//! Configuration sources and raw entries.

use crate::value::{Span, Value};

/// Precedence band of a configuration origin.
///
/// Highest priority first. Inside a band the application order of the source decides:
/// the last one wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Tier {
    /// Test or programmatic overrides.
    Override = 0,
    /// Command line arguments.
    Cli = 1,
    /// System environment variables.
    Environment = 2,
    /// Files of the active profile.
    ProfileFile = 3,
    /// Base files.
    BaseFile = 4,
    /// Defaults declared in the schema.
    SchemaDefault = 5,
    /// Defaults declared in code.
    CodeDefault = 6,
}

impl Tier {
    /// Band number. Lower is more important.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self as u8
    }
}

/// Identity of a source inside a build.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceId {
    /// Logical name: `file:app.yaml`, `env`, `cli`.
    pub name: String,
    /// Precedence band.
    pub tier: Tier,
    /// Application order inside the band.
    pub order: u32,
}

/// Raw entry extracted from a source, before the merge.
#[derive(Debug, Clone)]
pub struct RawEntry {
    /// Normalised path: `db.host`, `servers[0].host`.
    pub path: String,
    /// Value as it appeared in the source.
    pub value: Value,
    /// Position in the document, when the source knows it.
    pub span: Option<Span>,
}

/// Every entry of a source.
#[derive(Debug, Clone)]
pub struct RawSource {
    /// Source identity.
    pub id: SourceId,
    /// Entries in order of appearance.
    pub entries: Vec<RawEntry>,
}

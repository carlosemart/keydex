//! Sources, precedence tiers and typed paths.

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

/// One step of a configuration path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment {
    /// Object key.
    Key(String),
    /// Sequence index.
    Index(usize),
}

/// Typed configuration path.
///
/// A path is a list of segments, not a string: a literal key containing a dot or a bracket
/// cannot collide with a nested level.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Path {
    segments: Vec<Segment>,
}

impl Path {
    /// Empty path (the root).
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// Builds a path from its segments.
    #[must_use]
    pub fn from_segments(segments: Vec<Segment>) -> Self {
        Self { segments }
    }

    /// Parses `db.host`, `servers[0].host`, or a literal key with dots as `a["a.b"]`.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let mut segments = Vec::new();
        let mut buffer = String::new();
        let mut chars = input.chars().peekable();

        while let Some(current) = chars.next() {
            match current {
                '.' => {
                    if !buffer.is_empty() {
                        segments.push(Segment::Key(std::mem::take(&mut buffer)));
                    }
                }
                '[' => match chars.peek() {
                    Some('"') => {
                        chars.next();
                        let mut literal = String::new();
                        let mut escaped = false;
                        for inner in chars.by_ref() {
                            if escaped {
                                literal.push(inner);
                                escaped = false;
                                continue;
                            }
                            match inner {
                                '\\' => escaped = true,
                                '"' => break,
                                other => literal.push(other),
                            }
                        }
                        if chars.peek() == Some(&']') {
                            chars.next();
                        }
                        if !buffer.is_empty() {
                            segments.push(Segment::Key(std::mem::take(&mut buffer)));
                        }
                        segments.push(Segment::Key(literal));
                    }
                    _ => {
                        let mut digits = String::new();
                        for inner in chars.by_ref() {
                            if inner == ']' {
                                break;
                            }
                            digits.push(inner);
                        }
                        if !buffer.is_empty() {
                            segments.push(Segment::Key(std::mem::take(&mut buffer)));
                        }
                        if let Ok(index) = digits.parse::<usize>() {
                            segments.push(Segment::Index(index));
                        }
                    }
                },
                '"' => {
                    let mut literal = String::new();
                    let mut escaped = false;
                    for inner in chars.by_ref() {
                        if escaped {
                            literal.push(inner);
                            escaped = false;
                            continue;
                        }
                        match inner {
                            '\\' => escaped = true,
                            '"' => break,
                            other => literal.push(other),
                        }
                    }
                    segments.push(Segment::Key(literal));
                }
                other => buffer.push(other),
            }
        }

        if !buffer.is_empty() {
            segments.push(Segment::Key(buffer));
        }

        Self { segments }
    }

    /// Segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Appends a key.
    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.segments.push(Segment::Key(key.into()));
        self
    }

    /// Appends an index.
    #[must_use]
    pub fn with_index(mut self, index: usize) -> Self {
        self.segments.push(Segment::Index(index));
        self
    }

    /// `true` when the path points at the root.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&render_path(&self.segments))
    }
}

/// Renders a path from its segments, used for error messages, interpolation keys and the
/// flat key view.
///
/// Keys that contain a dot, a bracket or a quote are rendered as `["a.b"]`, so a literal
/// key can never collide with a nested path and `Path::parse` round-trips.
#[must_use]
pub fn render_path(segments: &[Segment]) -> String {
    let mut out = String::new();
    for segment in segments {
        match segment {
            Segment::Key(key) if needs_quoting(key) => {
                out.push_str("[\"");
                for character in key.chars() {
                    if matches!(character, '"' | '\\') {
                        out.push('\\');
                    }
                    out.push(character);
                }
                out.push_str("\"]");
            }
            Segment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            Segment::Index(index) => {
                out.push('[');
                out.push_str(&index.to_string());
                out.push(']');
            }
        }
    }
    out
}

fn needs_quoting(key: &str) -> bool {
    key.is_empty() || key.contains(['.', '[', ']', '"'])
}

/// A configuration source.
///
/// The engine knows nothing about how a source obtains its data: files, environment,
/// Vault or a test fixture all look the same from here. `read` consumes the box so a
/// source can move its tree out instead of cloning it.
pub trait Source: Send {
    /// Identity used for provenance: name, band and order.
    fn id(&self) -> SourceId;

    /// Produces the tree contributed by this source.
    ///
    /// # Errors
    ///
    /// Returns an error when the source cannot be read or parsed.
    fn read(self: Box<Self>) -> crate::error::Result<crate::value::Node>;
}

/// A built-in source: a tree that was already parsed.
#[derive(Debug)]
pub struct ParsedSource {
    id: SourceId,
    root: crate::value::Node,
}

impl ParsedSource {
    /// Wraps an already parsed tree.
    #[must_use]
    pub fn new(id: SourceId, root: crate::value::Node) -> Self {
        Self { id, root }
    }
}

impl Source for ParsedSource {
    fn id(&self) -> SourceId {
        self.id.clone()
    }

    fn read(self: Box<Self>) -> crate::error::Result<crate::value::Node> {
        Ok(self.root)
    }
}

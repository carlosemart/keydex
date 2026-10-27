//! Structured engine errors.

/// Engine result.
pub type Result<T> = core::result::Result<T, Error>;

/// Engine error.
///
/// Always structured: each layer decides how to format it (an exception in Python, JSON
/// in the CLI). A plain `String` is never returned as an error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A source cannot be read or does not exist.
    #[error("source unavailable: {name}: {reason}")]
    SourceUnavailable {
        /// Logical source name.
        name: String,
        /// Human readable reason.
        reason: String,
    },

    /// The document format could not be inferred.
    #[error("unknown format for {file}")]
    UnknownFormat {
        /// File involved.
        file: String,
    },

    /// The parser rejected the document.
    #[error("{file}: {message}")]
    Parse {
        /// File involved.
        file: String,
        /// Parser message.
        message: String,
    },

    /// The format is disabled by feature flags in this build.
    #[error("format not available in this build: {format}")]
    UnsupportedFormat {
        /// Format name.
        format: String,
    },

    /// The IR version is not the one this build understands.
    #[error("unsupported schema version: found {found}, expected {expected}")]
    UnsupportedSchema {
        /// Version found in the document.
        found: String,
        /// Version the engine understands.
        expected: String,
    },

    /// A `${...}` reference points to a key that does not exist.
    #[error("missing reference: {reference} (used in {path})")]
    MissingReference {
        /// Key where the placeholder appears.
        path: String,
        /// Reference that does not resolve.
        reference: String,
    },

    /// Cycle in the interpolation graph.
    #[error("interpolation cycle: {cycle}")]
    InterpolationCycle {
        /// Cycle path, separated by ` -> `.
        cycle: String,
    },

    /// Placeholder with invalid syntax.
    #[error("malformed placeholder in {path}: {snippet}")]
    MalformedPlaceholder {
        /// Key where the placeholder appears.
        path: String,
        /// Offending fragment.
        snippet: String,
    },

    /// Two schema paths tie for the same environment variable.
    #[error("ambiguous binding for {env_var}: {candidates:?}")]
    AmbiguousBinding {
        /// Environment variable involved.
        env_var: String,
        /// Schema paths that tie.
        candidates: Vec<String>,
    },
}

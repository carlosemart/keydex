//! # keydex-core
//!
//! keydex configuration engine: sources, precedence, merge, interpolation, coercion and
//! provenance.
//!
//! This crate is pure Rust and is published on crates.io. Hard rules (see `AGENTS.md`):
//!
//! - It does **not** depend on `pyo3`.
//! - It does **not** read the environment, the clock or the filesystem: everything is
//!   injected.
//! - Errors are enums, never `String`.
//! - No mutable global state.

#![forbid(unsafe_code)]
#![forbid(static_mut_refs)]
#![deny(
    missing_docs,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_methods
)]

mod env;
mod error;
mod interp;
mod loader;
mod parse;
#[cfg(feature = "schema")]
mod schema;
mod snapshot;
mod source;
mod value;

pub use error::{Error, Result};
pub use loader::Builder;
pub use parse::{Format, LineIndex};
#[cfg(feature = "schema")]
pub use schema::{Field, IR_VERSION, Schema, UnknownKey, ValidationError, coerce, unknown_keys};
pub use snapshot::{Explanation, Snapshot};
pub use source::{RawEntry, RawSource, SourceId, Tier};
pub use value::{Provenance, Span, Value};

/// Version of the schema contract (`keydex.schema/v1`).
///
/// The IR is versioned independently from the package: it survives several releases and
/// only changes when the contract breaks.
pub const SCHEMA_IR_VERSION: &str = "keydex.schema/v1";

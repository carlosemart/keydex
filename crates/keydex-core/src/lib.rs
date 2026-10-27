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
//!
//! # Example
//!
//! ```
//! use keydex_core::{Builder, Format, Tier};
//!
//! let mut builder = Builder::new();
//! builder
//!     .document(
//!         "app.yaml",
//!         "db:\n  host: localhost\n  port: 5432\n  url: \"${db.host}:${db.port}\"\n",
//!         Format::Yaml,
//!         Tier::BaseFile,
//!     )
//!     .unwrap();
//!
//! let snapshot = builder.build().unwrap();
//!
//! assert_eq!(snapshot.get("db.host").unwrap().to_string(), "localhost");
//! // Interpolation is resolved once, at build time.
//! assert_eq!(snapshot.get("db.url").unwrap().to_string(), "localhost:5432");
//! // Every value knows where it came from.
//! assert_eq!(
//!     snapshot.explain("db.host").unwrap().winner.source,
//!     "file:app.yaml"
//! );
//! ```
//!
//! A sequence in a higher-priority source replaces the whole sequence, because the value
//! model is a tree:
//!
//! ```
//! use keydex_core::{Builder, Format, Tier};
//!
//! let mut builder = Builder::new();
//! builder
//!     .document("base.yaml", "tags:\n  - a\n  - b\n", Format::Yaml, Tier::BaseFile)
//!     .unwrap();
//! builder
//!     .document("prod.yaml", "tags:\n  - c\n", Format::Yaml, Tier::ProfileFile)
//!     .unwrap();
//!
//! let snapshot = builder.build().unwrap();
//! assert_eq!(snapshot.get("tags").unwrap().to_string(), "[c]");
//! ```
//!
//! See `examples/basic.rs` and `examples/schema.rs` for complete programs.

#![forbid(unsafe_code)]
#![forbid(static_mut_refs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
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
mod merge;
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
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub use schema::{Field, IR_VERSION, Schema, UnknownKey, ValidationError, coerce, unknown_keys};
pub use snapshot::{Explanation, Snapshot};
pub use source::{Path, Segment, SourceId, Tier};
pub use value::{Node, Provenance, Span, Value};

/// Version of the schema contract (`keydex.schema/v1`).
///
/// The IR is versioned independently from the package: it survives several releases and
/// only changes when the contract breaks.
pub const SCHEMA_IR_VERSION: &str = "keydex.schema/v1";

//! # keydex-core
//!
//! Motor de configuración de `keydex`: fuentes, precedencia, merge, interpolación y
//! procedencia.
//!
//! Este crate es Rust puro y se publica en crates.io. Reglas duras (ver `AGENTS.md`):
//!
//! - **No** depende de `pyo3`.
//! - **No** lee entorno, reloj ni sistema de ficheros: todo entra inyectado.
//! - Los errores son enums, nunca `String`.
//! - Cero estado global mutable.

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

/// Versión del contrato del esquema (`keydex.schema/v1`).
///
/// El IR se versiona de forma independiente a la versión del paquete: sobrevive a
/// varias releases y solo cambia ante una ruptura del contrato.
pub const SCHEMA_IR_VERSION: &str = "keydex.schema/v1";

//! # keydex-core
//!
//! Motor de configuración de `keydex`: fuentes, precedencia, merge, interpolación,
//! coerción y procedencia.
//!
//! Este crate es Rust puro y se publica en crates.io. Reglas duras (ver `AGENTS.md`):
//!
//! - **No** depende de `pyo3`.
//! - **No** lee entorno, reloj ni sistema de ficheros: todo entra inyectado
//!   (`EnvSource`, `FileProvider`, `Clock`).
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

/// Versión del contrato del esquema (`keydex.schema/v1`).
///
/// El IR se versiona de forma independiente a la versión del paquete: sobrevive a
/// varias releases y solo cambia ante una ruptura del contrato.
pub const SCHEMA_IR_VERSION: &str = "keydex.schema/v1";

/// Banda de precedencia de un origen de configuración.
///
/// De mayor a menor prioridad. Dentro de una misma banda decide el orden de
/// inserción de la fuente: la última en aplicarse gana.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Tier {
    /// Overrides de test o programáticos.
    Override = 0,
    /// Argumentos de línea de comandos.
    Cli = 1,
    /// Variables de entorno del sistema.
    Environment = 2,
    /// Ficheros del perfil activo.
    ProfileFile = 3,
    /// Ficheros base.
    BaseFile = 4,
    /// Defaults declarados en el esquema.
    SchemaDefault = 5,
    /// Defaults declarados en el código.
    CodeDefault = 6,
}

impl Tier {
    /// Número de banda. Menor es más prioritario.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self as u8
    }
}

/// Error del motor.
///
/// Todos los errores son estructurados: cada capa los formatea (excepción en
/// Python, JSON en la CLI). Nunca se devuelve un `String` como error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Una fuente declarada no existe o no se puede leer.
    #[error("origen no disponible: {name}")]
    SourceUnavailable {
        /// Nombre lógico del origen.
        name: String,
    },

    /// El binding de una variable de entorno es ambiguo contra el esquema.
    #[error("binding ambiguo para la variable {env_var}: {candidates:?}")]
    AmbiguousBinding {
        /// Variable de entorno implicada.
        env_var: String,
        /// Rutas del esquema que empatan.
        candidates: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_version_del_ir_es_v1() {
        assert_eq!(SCHEMA_IR_VERSION, "keydex.schema/v1");
    }

    #[test]
    fn las_bandas_estan_ordenadas_por_prioridad() {
        assert!(Tier::Override < Tier::Cli);
        assert!(Tier::Cli < Tier::Environment);
        assert!(Tier::Environment < Tier::ProfileFile);
        assert!(Tier::ProfileFile < Tier::BaseFile);
        assert_eq!(Tier::Override.rank(), 0);
        assert_eq!(Tier::CodeDefault.rank(), 6);
    }
}

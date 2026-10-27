//! Errores estructurados del motor.

/// Resultado del motor.
pub type Result<T> = core::result::Result<T, Error>;

/// Error del motor.
///
/// Siempre estructurado: cada capa decide cómo formatearlo (excepción en Python,
/// JSON en la CLI). Nunca se devuelve un `String` como error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Una fuente no se puede leer o no existe.
    #[error("origen no disponible: {name}: {reason}")]
    SourceUnavailable {
        /// Nombre lógico de la fuente.
        name: String,
        /// Motivo legible.
        reason: String,
    },

    /// No se pudo deducir el formato del documento.
    #[error("formato desconocido para {file}")]
    UnknownFormat {
        /// Fichero implicado.
        file: String,
    },

    /// El parser rechazó el documento.
    #[error("{file}: {message}")]
    Parse {
        /// Fichero implicado.
        file: String,
        /// Mensaje del parser.
        message: String,
    },

    /// El formato está desactivado por feature flags en este build.
    #[error("formato no disponible en este build: {format}")]
    UnsupportedFormat {
        /// Nombre del formato.
        format: String,
    },

    /// Una referencia `${...}` apunta a una clave que no existe.
    #[error("referencia inexistente: {reference} (usada en {path})")]
    MissingReference {
        /// Clave donde aparece el placeholder.
        path: String,
        /// Referencia que no resuelve.
        reference: String,
    },

    /// Ciclo en el grafo de interpolación.
    #[error("ciclo de interpolación: {cycle}")]
    InterpolationCycle {
        /// Recorrido del ciclo, separado por ` -> `.
        cycle: String,
    },

    /// Placeholder con sintaxis inválida.
    #[error("placeholder mal formado en {path}: {snippet}")]
    MalformedPlaceholder {
        /// Clave donde aparece el placeholder.
        path: String,
        /// Fragmento ofensor.
        snippet: String,
    },
}

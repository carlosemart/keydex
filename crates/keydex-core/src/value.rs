//! Valores, posiciones y procedencia.

use std::fmt;

use crate::source::Tier;

/// Valor de configuración ya resuelto.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// Ausencia explícita (`null`).
    Null,
    /// Booleano.
    Bool(bool),
    /// Entero de 64 bits.
    Int(i64),
    /// Coma flotante.
    Float(f64),
    /// Cadena de texto.
    Str(String),
}

impl Value {
    /// Devuelve la cadena subyacente, si el valor es textual.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }

    /// `true` si el valor es textual.
    #[must_use]
    pub fn is_str(&self) -> bool {
        matches!(self, Self::Str(_))
    }

    /// Construye un valor textual.
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

/// Posición de un valor dentro de un documento.
///
/// La columna se expone siempre en **caracteres**, no en bytes: los parsers de TOML y
/// JSON trabajan con offsets de bytes y se normalizan al construir el span.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// Nombre del fichero tal y como lo declaró la fuente.
    pub file: String,
    /// Línea, 1-based.
    pub line: u32,
    /// Columna, 0-based, en caracteres.
    pub col: u32,
}

/// De dónde salió un valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Nombre lógico de la fuente (`file:app.yaml`, `env`).
    pub source: String,
    /// Banda de precedencia.
    pub tier: Tier,
    /// Orden de aplicación dentro de la banda.
    pub order: u32,
    /// Posición en el documento, si la fuente la conoce.
    pub span: Option<Span>,
}

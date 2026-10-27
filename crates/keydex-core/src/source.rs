//! Fuentes de configuración y entradas crudas.

use crate::value::{Span, Value};

/// Banda de precedencia de un origen de configuración.
///
/// De mayor a menor prioridad. Dentro de una misma banda decide el orden de aplicación
/// de la fuente: la última gana.
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

/// Identidad de una fuente dentro de un build.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceId {
    /// Nombre lógico: `file:app.yaml`, `env`, `cli`.
    pub name: String,
    /// Banda de precedencia.
    pub tier: Tier,
    /// Orden de aplicación dentro de la banda.
    pub order: u32,
}

/// Entrada cruda extraída de una fuente, antes del merge.
#[derive(Debug, Clone)]
pub struct RawEntry {
    /// Ruta normalizada: `db.host`, `servers[0].host`.
    pub path: String,
    /// Valor tal y como aparecía en la fuente.
    pub value: Value,
    /// Posición en el documento, si la fuente la conoce.
    pub span: Option<Span>,
}

/// Todas las entradas de una fuente.
#[derive(Debug, Clone)]
pub struct RawSource {
    /// Identidad de la fuente.
    pub id: SourceId,
    /// Entradas en orden de aparición.
    pub entries: Vec<RawEntry>,
}

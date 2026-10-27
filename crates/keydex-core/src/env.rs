//! Fuente de variables de entorno.

use std::collections::BTreeMap;

use crate::source::RawEntry;
use crate::value::Value;

/// Convierte variables de entorno ya capturadas en entradas planas.
///
/// Reglas del **modo diccionario** (sin esquema, ver especificación §6.2): el prefijo se
/// elimina sin distinguir mayúsculas, `__` separa niveles y `_` es un carácter literal.
/// No hay heurísticas: sin esquema no se adivina.
#[must_use]
pub fn parse(env: &BTreeMap<String, String>, prefix: &str) -> Vec<RawEntry> {
    let wanted = prefix.to_ascii_uppercase();
    let mut out = Vec::new();

    for (name, value) in env {
        let upper = name.to_ascii_uppercase();
        let Some(rest) = upper.strip_prefix(&wanted) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        out.push(RawEntry {
            path: rest.to_ascii_lowercase().replace("__", "."),
            value: Value::str(value.as_str()),
            span: None,
        });
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

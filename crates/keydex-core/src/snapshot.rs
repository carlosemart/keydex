//! Snapshot inmutable con procedencia.

use std::collections::BTreeMap;

use crate::value::{Provenance, Value};

/// Valor resuelto con su procedencia completa.
#[derive(Debug, Clone)]
pub(crate) struct Resolved {
    pub(crate) value: Value,
    pub(crate) provenance: Provenance,
    pub(crate) shadowed: Vec<Provenance>,
    pub(crate) refs: Vec<String>,
}

/// Explicación de una clave: el valor y por qué es ese.
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Clave consultada.
    pub key: String,
    /// Valor final.
    pub value: Value,
    /// Fuente ganadora.
    pub winner: Provenance,
    /// Fuentes que definieron la clave y perdieron, de la más antigua a la más reciente.
    pub shadowed: Vec<Provenance>,
    /// Claves referenciadas por interpolación.
    pub refs: Vec<String>,
}

/// Configuración resuelta e inmutable.
///
/// Es el único estado que comparten los hilos: no hay mutación ni bloqueos. Si en el
/// futuro hay `refresh()`, se construye otro snapshot y se intercambia el puntero.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    entries: BTreeMap<String, Resolved>,
}

impl Snapshot {
    pub(crate) fn new(entries: BTreeMap<String, Resolved>) -> Self {
        Self { entries }
    }

    /// Valor de una clave, si existe.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// Número de claves.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` si no hay claves.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Claves en orden lexicográfico.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Claves y valores en orden lexicográfico.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries
            .iter()
            .map(|(key, entry)| (key.as_str(), &entry.value))
    }

    /// Procedencia de una clave: quién ganó, quién fue sombreado y qué referencias usó.
    #[must_use]
    pub fn explain(&self, key: &str) -> Option<Explanation> {
        self.entries.get(key).map(|entry| Explanation {
            key: key.to_string(),
            value: entry.value.clone(),
            winner: entry.provenance.clone(),
            shadowed: entry.shadowed.clone(),
            refs: entry.refs.clone(),
        })
    }
}

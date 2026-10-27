//! Immutable snapshot with provenance.

use std::collections::BTreeMap;

use crate::value::{Provenance, Value};

/// Resolved value with its full provenance.
#[derive(Debug, Clone)]
pub(crate) struct Resolved {
    pub(crate) value: Value,
    pub(crate) provenance: Provenance,
    pub(crate) shadowed: Vec<Provenance>,
    pub(crate) refs: Vec<String>,
}

/// Explanation of a key: the value and why it is that value.
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Queried key.
    pub key: String,
    /// Final value.
    pub value: Value,
    /// Winning source.
    pub winner: Provenance,
    /// Sources that defined the key and lost, oldest first.
    pub shadowed: Vec<Provenance>,
    /// Keys referenced through interpolation.
    pub refs: Vec<String>,
}

/// Resolved and immutable configuration.
///
/// It is the only state shared between threads: no mutation, no locks. A future
/// `refresh()` builds another snapshot and swaps the pointer.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    entries: BTreeMap<String, Resolved>,
}

impl Snapshot {
    pub(crate) fn new(entries: BTreeMap<String, Resolved>) -> Self {
        Self { entries }
    }

    /// Value of a key, when it exists.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// Number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when there are no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Keys in lexicographic order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Keys and values in lexicographic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries
            .iter()
            .map(|(key, entry)| (key.as_str(), &entry.value))
    }

    /// Keys starting with a prefix, in lexicographic order.
    ///
    /// Used to rebuild lists from the flattened keys (`servers[0]`, `servers[1]`, ...).
    pub fn keys_with_prefix<'a>(&'a self, prefix: &str) -> impl Iterator<Item = &'a str> + 'a {
        let start = prefix.to_string();
        self.entries
            .range(start.clone()..)
            .take_while(move |(key, _)| key.starts_with(&start))
            .map(|(key, _)| key.as_str())
    }

    /// Provenance of a key: who won, who was shadowed and which references it used.
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

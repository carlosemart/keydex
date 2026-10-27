//! Snapshot building.
//!
//! The `Builder` does not read files or the environment: it receives the contents and the
//! captured variables. Reading is the responsibility of the calling layer.

use std::collections::BTreeMap;

use crate::env;
use crate::error::Result;
use crate::interp;
use crate::parse::{self, Format};
use crate::snapshot::{Resolved, Snapshot};
use crate::source::{RawEntry, RawSource, SourceId, Tier};
use crate::value::{Provenance, Value};

/// Snapshot builder.
#[derive(Debug, Default)]
pub struct Builder {
    sources: Vec<RawSource>,
    env: BTreeMap<String, String>,
    next_order: u32,
}

impl Builder {
    /// Empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the already captured environment variables.
    ///
    /// The core does not read `std::env`: the caller decides which environment the
    /// configuration sees.
    #[must_use]
    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Adds an environment variable source.
    pub fn env_source(&mut self, prefix: &str, tier: Tier) -> &mut Self {
        let entries = env::parse(&self.env, prefix);
        let order = self.take_order();
        self.sources.push(RawSource {
            id: SourceId {
                name: String::from("env"),
                tier,
                order,
            },
            entries,
        });
        self
    }

    /// Adds an environment source with **relaxed binding against the schema**.
    ///
    /// Unlike [`Builder::env_source`], here `APP_DB_HOST` matches `db.host` without
    /// needing `__`: the schema disambiguates.
    #[cfg(feature = "schema")]
    pub fn schema_env_source(
        &mut self,
        schema: &crate::schema::Schema,
        prefix: &str,
        tier: Tier,
    ) -> Result<&mut Self> {
        let bindings = crate::schema::bind_env(&schema.root, &self.env, prefix)?;
        let entries = bindings
            .into_iter()
            .map(|(path, value)| RawEntry {
                path,
                value: Value::str(value),
                span: None,
            })
            .collect();
        let order = self.take_order();
        self.sources.push(RawSource {
            id: SourceId {
                name: String::from("env"),
                tier,
                order,
            },
            entries,
        });
        Ok(self)
    }

    /// Adds a document already read into memory.
    pub fn document(
        &mut self,
        name: &str,
        contents: &str,
        format: Format,
        tier: Tier,
    ) -> Result<&mut Self> {
        let entries = parse::parse(contents, name, format)?;
        let order = self.take_order();
        self.sources.push(RawSource {
            id: SourceId {
                name: format!("file:{name}"),
                tier,
                order,
            },
            entries,
        });
        Ok(self)
    }

    /// Applies precedence, resolves interpolations and returns the snapshot.
    pub fn build(self) -> Result<Snapshot> {
        let Builder {
            mut sources, env, ..
        } = self;
        // Sources are applied from LOWEST to HIGHEST priority: band descending (a high
        // rank means less important) and, inside the band, order ascending. The last one
        // to be applied wins.
        sources.sort_by(|a, b| {
            b.id.tier
                .rank()
                .cmp(&a.id.tier.rank())
                .then_with(|| a.id.order.cmp(&b.id.order))
        });

        let mut merged: BTreeMap<String, (Value, Provenance, Vec<Provenance>)> = BTreeMap::new();
        for source in sources {
            for entry in source.entries {
                let provenance = Provenance {
                    source: source.id.name.clone(),
                    tier: source.id.tier,
                    order: source.id.order,
                    span: entry.span,
                };
                match merged.get_mut(&entry.path) {
                    Some((value, winner, shadowed)) => {
                        shadowed.push(winner.clone());
                        *value = entry.value;
                        *winner = provenance;
                    }
                    None => {
                        merged.insert(entry.path, (entry.value, provenance, Vec::new()));
                    }
                }
            }
        }

        let raw: BTreeMap<String, Value> = merged
            .iter()
            .map(|(path, (value, _, _))| (path.clone(), value.clone()))
            .collect();

        let (values, refs) = interp::resolve_all(&raw, &env)?;

        let mut entries = BTreeMap::new();
        for (path, (_, provenance, shadowed)) in merged {
            let Some(value) = values.get(&path).cloned() else {
                continue;
            };
            let entry = Resolved {
                value,
                provenance,
                shadowed,
                refs: refs.get(&path).cloned().unwrap_or_default(),
            };
            entries.insert(path, entry);
        }

        Ok(Snapshot::new(entries))
    }

    fn take_order(&mut self) -> u32 {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        order
    }
}

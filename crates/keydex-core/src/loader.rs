//! Construcción de snapshots.
//!
//! El `Builder` no lee ficheros ni entorno: recibe el contenido y el mapa de variables
//! ya capturados. La lectura es responsabilidad de la capa que lo llama.

use std::collections::BTreeMap;

use crate::env;
use crate::error::Result;
use crate::interp;
use crate::parse::{self, Format};
use crate::snapshot::{Resolved, Snapshot};
use crate::source::{RawEntry, RawSource, SourceId, Tier};
use crate::value::{Provenance, Value};

/// Constructor de snapshots.
#[derive(Debug, Default)]
pub struct Builder {
    sources: Vec<RawSource>,
    env: BTreeMap<String, String>,
    next_order: u32,
}

impl Builder {
    /// Constructor vacío.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fija las variables de entorno ya capturadas.
    ///
    /// El core no lee `std::env`: el llamante decide qué entorno ve la configuración.
    #[must_use]
    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Añade una fuente de variables de entorno.
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

    /// Añade una fuente de entorno con **binding relajado contra el esquema**.
    ///
    /// A diferencia de [`Builder::env_source`], aquí `APP_DB_HOST` casa con `db.host`
    /// sin necesidad de `__`: el esquema desambigua.
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

    /// Añade un documento ya leído en memoria.
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

    /// Aplica precedencia, resuelve interpolaciones y devuelve el snapshot.
    pub fn build(self) -> Result<Snapshot> {
        let Builder {
            mut sources, env, ..
        } = self;
        // Se aplican de MENOR a MAYOR prioridad: banda descendente (rank alto = menos
        // prioritario) y, dentro de la banda, orden ascendente. La última en aplicarse
        // gana.
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

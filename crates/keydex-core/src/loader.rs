//! Snapshot building.
//!
//! The `Builder` does not read files or the environment: it receives the contents and the
//! captured variables. Reading is the responsibility of the calling layer.

use std::collections::BTreeMap;

use crate::Snapshot;
use crate::env;
use crate::error::Result;
use crate::interp;
use crate::merge::{insert, merge};
use crate::parse::{self, Format};
use crate::source::{Path, Segment, SourceId, Tier};
use crate::value::{Node, Provenance, Value};

/// One source and the tree it produced.
#[derive(Debug, Clone)]
struct SourceTree {
    id: SourceId,
    root: Node,
}

/// Snapshot builder.
#[derive(Debug, Default)]
pub struct Builder {
    sources: Vec<SourceTree>,
    env: BTreeMap<String, String>,
    sensitive: Vec<Path>,
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
        let order = self.take_order();
        let id = SourceId {
            name: String::from("env"),
            tier,
            order,
        };
        let mut root = Node::empty_map();
        for (path, value) in env::parse(&self.env, prefix) {
            let provenance = Provenance {
                source: id.name.clone(),
                tier,
                order,
                span: None,
            };
            root = insert(root, path.segments(), value, provenance);
        }
        self.sources.push(SourceTree { id, root });
        self
    }

    /// Adds an environment source with **relaxed binding against the schema**.
    ///
    /// Unlike [`Builder::env_source`], here `APP_DB_HOST` matches `db.host` without
    /// needing `__`: the schema disambiguates.
    #[cfg(feature = "schema")]
    #[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
    pub fn schema_env_source(
        &mut self,
        schema: &crate::schema::Schema,
        prefix: &str,
        tier: Tier,
    ) -> Result<&mut Self> {
        let order = self.take_order();
        let id = SourceId {
            name: String::from("env"),
            tier,
            order,
        };
        let mut root = Node::empty_map();
        for (path, value) in crate::schema::bind_env(&schema.root, &self.env, prefix)? {
            let provenance = Provenance {
                source: id.name.clone(),
                tier,
                order,
                span: None,
            };
            root = insert(
                root,
                Path::parse(&path).segments(),
                Value::str(value),
                provenance,
            );
        }
        self.sources.push(SourceTree { id, root });
        Ok(self)
    }

    /// Marks a key and its subtree as sensitive.
    ///
    /// Sensitivity travels with the node: it survives the merge, propagates through
    /// interpolation, and makes every diagnostic output redact it by default.
    pub fn mark_sensitive(&mut self, key: &str) -> &mut Self {
        self.sensitive.push(Path::parse(key));
        self
    }

    /// Adds a document already read into memory.
    pub fn document(
        &mut self,
        name: &str,
        contents: &str,
        format: Format,
        tier: Tier,
    ) -> Result<&mut Self> {
        let order = self.take_order();
        let id = SourceId {
            name: format!("file:{name}"),
            tier,
            order,
        };

        let mut root = Node::empty_map();
        for document in parse::parse(contents, name, format)? {
            let mut document = document;
            stamp(&mut document, &id);
            root = merge(root, document);
        }

        self.sources.push(SourceTree { id, root });
        Ok(self)
    }

    /// Applies precedence, resolves interpolations and returns the snapshot.
    pub fn build(self) -> Result<Snapshot> {
        let Builder {
            mut sources,
            env,
            sensitive,
            ..
        } = self;

        // Lowest priority first: band descending, order ascending. The last one wins.
        sources.sort_by(|a, b| {
            b.id.tier
                .rank()
                .cmp(&a.id.tier.rank())
                .then_with(|| a.id.order.cmp(&b.id.order))
        });

        let mut root = Node::empty_map();
        for source in sources {
            root = merge(root, source.root);
        }

        for path in &sensitive {
            if let Some(node) = node_at_mut(&mut root, path.segments()) {
                mark_subtree(node);
            }
        }

        interp::resolve_tree(&mut root, &env)?;
        Ok(Snapshot::new(root))
    }

    fn take_order(&mut self) -> u32 {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        order
    }
}

/// Attaches the source's provenance to every node of the tree.
fn stamp(node: &mut Node, id: &SourceId) {
    node.provenance = Some(Provenance {
        source: id.name.clone(),
        tier: id.tier,
        order: id.order,
        span: node.span.clone(),
    });

    match &mut node.value {
        Value::Map(entries) => {
            for child in entries.values_mut() {
                stamp(child, id);
            }
        }
        Value::Seq(items) => {
            for child in items.iter_mut() {
                stamp(child, id);
            }
        }
        _ => {}
    }
}

/// Navigates to a node, creating nothing.
fn node_at_mut<'a>(node: &'a mut Node, segments: &[Segment]) -> Option<&'a mut Node> {
    let Some((head, rest)) = segments.split_first() else {
        return Some(node);
    };
    match (head, &mut node.value) {
        (Segment::Key(key), Value::Map(entries)) => entries
            .get_mut(key)
            .and_then(|child| node_at_mut(child, rest)),
        (Segment::Index(index), Value::Seq(items)) => items
            .get_mut(*index)
            .and_then(|child| node_at_mut(child, rest)),
        _ => None,
    }
}

/// Marks a node and everything under it as sensitive.
fn mark_subtree(node: &mut Node) {
    node.sensitive = true;
    match &mut node.value {
        Value::Map(entries) => {
            for child in entries.values_mut() {
                mark_subtree(child);
            }
        }
        Value::Seq(items) => {
            for child in items.iter_mut() {
                mark_subtree(child);
            }
        }
        _ => {}
    }
}

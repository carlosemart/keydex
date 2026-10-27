//! Snapshot building.
//!
//! The `Builder` does not read files or the environment: it receives contents and a
//! captured environment map. Reading is the responsibility of the caller or of a custom
//! [`Source`] implementation.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::Snapshot;
use crate::env;
use crate::error::Result;
use crate::interp;
use crate::merge::{insert, merge};
use crate::parse::{self, Format};
use crate::resolve::{Resolver, Resolvers};
use crate::source::{Path, Segment, Source, SourceId, Tier};
use crate::value::{Node, Provenance, Value};
use crate::walk;

/// Snapshot builder.
pub struct Builder {
    sources: Vec<Box<dyn Source>>,
    env: BTreeMap<String, String>,
    resolvers: Resolvers,
    sensitive: Vec<Path>,
    sensitive_env: std::collections::BTreeSet<String>,
    next_order: u32,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("sources", &self.sources.len())
            .field("resolvers", &self.resolvers)
            .field("sensitive", &self.sensitive)
            .finish()
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            env: BTreeMap::new(),
            resolvers: Resolvers::with_builtins(),
            sensitive: Vec::new(),
            sensitive_env: std::collections::BTreeSet::new(),
            next_order: 0,
        }
    }
}

impl Builder {
    /// Empty builder with the built-in resolvers.
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

    /// Registers a resolver for `${name:...}`.
    ///
    /// A resolver replaces the built-in one with the same name.
    pub fn resolver(&mut self, resolver: Arc<dyn Resolver>) -> &mut Self {
        self.resolvers.register(resolver);
        self
    }

    /// Adds a custom source.
    ///
    /// The source decides its own [`SourceId`], so it can pick a tier and an order.
    pub fn source(&mut self, source: Box<dyn Source>) -> &mut Self {
        self.sources.push(source);
        self
    }

    /// Adds an environment variable source.
    pub fn env_source(&mut self, prefix: &str, tier: Tier) -> &mut Self {
        let id = SourceId {
            name: String::from("env"),
            tier,
            order: self.take_order(),
        };
        let mut root = Node::empty_map();
        for (path, value, name) in env::parse(&self.env, prefix) {
            let provenance = Provenance {
                source: id.name.clone(),
                tier,
                order: id.order,
                span: None,
            };
            let sensitive = self.sensitive_env.contains(&name.to_ascii_uppercase());
            let segments = path.segments().to_vec();
            root = insert(root, &segments, value, provenance);
            if sensitive {
                if let Some(node) = node_at_mut(&mut root, &segments) {
                    mark_subtree(node);
                }
            }
        }
        self.sources
            .push(Box::new(crate::source::ParsedSource::new(id, root)));
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
        let id = SourceId {
            name: String::from("env"),
            tier,
            order: self.take_order(),
        };
        let mut root = Node::empty_map();
        for (path, value, name) in crate::schema::bind_env(&schema.root, &self.env, prefix)? {
            let provenance = Provenance {
                source: id.name.clone(),
                tier,
                order: id.order,
                span: None,
            };
            let sensitive = self.sensitive_env.contains(&name.to_ascii_uppercase());
            let segments = Path::parse(&path).segments().to_vec();
            root = insert(root, &segments, Value::str(value), provenance);
            if sensitive {
                if let Some(node) = node_at_mut(&mut root, &segments) {
                    mark_subtree(node);
                }
            }
        }
        self.sources
            .push(Box::new(crate::source::ParsedSource::new(id, root)));
        Ok(self)
    }

    /// Marks as sensitive every key bound from this environment variable.
    ///
    /// The mapping from variable to path is done by the same binder that resolved it, so
    /// relaxed binding cannot desynchronise the mark. Call it **before** the environment
    /// source is added: the mark is applied while the value is inserted, and a later call
    /// is a silent no-op.
    pub fn mark_sensitive_env(&mut self, name: &str) -> &mut Self {
        self.sensitive_env.insert(name.to_ascii_uppercase());
        self
    }

    /// Marks a key and its subtree as sensitive.
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
        let id = SourceId {
            name: format!("file:{name}"),
            tier,
            order: self.take_order(),
        };

        let mut root = Node::empty_map();
        for document in parse::parse(contents, name, format)? {
            root = merge(root, document);
        }

        self.sources
            .push(Box::new(crate::source::ParsedSource::new(id, root)));
        Ok(self)
    }

    /// Applies precedence, resolves interpolations and returns the snapshot.
    pub fn build(self) -> Result<Snapshot> {
        let Builder {
            sources,
            env,
            resolvers,
            sensitive,
            ..
        } = self;

        let mut trees: Vec<(SourceId, Node)> = Vec::with_capacity(sources.len());
        for source in sources {
            let id = source.id();
            let mut root = source.read()?;
            // Provenance is the builder's job, so a custom source does not have to know
            // about it.
            stamp(&mut root, &id);
            trees.push((id, root));
        }

        // Lowest priority first: band descending, order ascending. The last one wins.
        trees.sort_by(|(a, _), (b, _)| {
            b.tier
                .rank()
                .cmp(&a.tier.rank())
                .then_with(|| a.order.cmp(&b.order))
        });

        let mut root = Node::empty_map();
        for (_, tree) in trees {
            root = merge(root, tree);
        }

        for path in &sensitive {
            if let Some(node) = node_at_mut(&mut root, path.segments()) {
                mark_subtree(node);
            }
        }

        interp::resolve_tree(&mut root, &env, &resolvers)?;
        Ok(Snapshot::new(root))
    }

    fn take_order(&mut self) -> u32 {
        let order = self.next_order;
        self.next_order = self.next_order.saturating_add(1);
        order
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
    walk::visit_mut(node, &mut Vec::new(), &mut |_, child| {
        child.sensitive = true;
    });
}

/// Attaches the source's provenance to every node of the tree.
fn stamp(node: &mut Node, id: &SourceId) {
    walk::visit_mut(node, &mut Vec::new(), &mut |_, child| {
        child.provenance = Some(Provenance {
            source: id.name.clone(),
            tier: id.tier,
            order: id.order,
            span: child.span.clone(),
        });
    });
}

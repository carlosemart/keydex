//! Immutable snapshot with provenance.

use crate::source::{Path, Segment, render_path};
use crate::value::{Node, Provenance, Value};
use crate::walk;

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

/// A leaf, with its rendered path and its segments, so iteration does not re-parse.
#[derive(Debug, Clone)]
struct Leaf {
    rendered: String,
    segments: Vec<Segment>,
}

/// Resolved and immutable configuration.
///
/// It holds the merged tree plus its leaves. It is the only state shared between threads.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    root: Node,
    leaves: Vec<Leaf>,
}

impl Snapshot {
    pub(crate) fn new(mut root: Node) -> Self {
        // A container that holds a secret is itself sensitive: otherwise reading `db`
        // returns the password in the clear.
        mark_containers(&mut root);

        let mut leaves = Vec::new();
        let mut path = Vec::new();
        walk::visit(&root, &mut path, &mut |segments, node| {
            if !node.value.is_container() {
                leaves.push(Leaf {
                    rendered: render_path(segments),
                    segments: segments.to_vec(),
                });
            }
        });
        Self { root, leaves }
    }

    pub(crate) fn root(&self) -> &Node {
        &self.root
    }

    /// Node at a path, containers included.
    #[must_use]
    pub fn node(&self, key: &str) -> Option<&Node> {
        navigate(&self.root, Path::parse(key).segments())
    }

    /// Value at a path, containers included.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.node(key).map(|node| &node.value)
    }

    /// Number of leaf keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// `true` when there are no leaf keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Leaf keys in lexicographic order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.leaves.iter().map(|leaf| leaf.rendered.as_str())
    }

    /// Leaf keys and values in lexicographic order, without re-parsing the paths.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.leaves.iter().filter_map(|leaf| {
            navigate(&self.root, &leaf.segments).map(|node| (leaf.rendered.as_str(), &node.value))
        })
    }

    /// `true` when the value must be hidden from diagnostic output.
    #[must_use]
    pub fn is_sensitive(&self, key: &str) -> bool {
        self.node(key).is_some_and(|node| node.sensitive)
    }

    /// Provenance of a key: who won, who was shadowed and which references it used.
    #[must_use]
    pub fn explain(&self, key: &str) -> Option<Explanation> {
        let node = self.node(key)?;
        Some(Explanation {
            key: key.to_string(),
            value: node.value.clone(),
            winner: node.provenance.clone()?,
            shadowed: node.shadowed.clone(),
            refs: node.refs.clone(),
        })
    }
}

/// Marks every node whose subtree contains a sensitive leaf.
fn mark_containers(node: &mut Node) -> bool {
    let mut any = node.sensitive;
    match &mut node.value {
        Value::Map(entries) => {
            for child in entries.values_mut() {
                any |= mark_containers(child);
            }
        }
        Value::Seq(items) => {
            for child in items.iter_mut() {
                any |= mark_containers(child);
            }
        }
        _ => {}
    }
    if any {
        node.sensitive = true;
    }
    any
}

fn navigate<'a>(node: &'a Node, segments: &[Segment]) -> Option<&'a Node> {
    let Some((head, rest)) = segments.split_first() else {
        return Some(node);
    };
    match (head, &node.value) {
        (Segment::Key(key), Value::Map(map)) => {
            map.get(key).and_then(|child| navigate(child, rest))
        }
        (Segment::Index(index), Value::Seq(items)) => {
            items.get(*index).and_then(|child| navigate(child, rest))
        }
        _ => None,
    }
}

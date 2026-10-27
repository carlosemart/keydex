//! Immutable snapshot with provenance.

use crate::source::{Path, Segment, render_path};
use crate::value::{Node, Provenance, Value};

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
/// It holds the merged tree plus the list of leaf paths, so `keys()` is deterministic and
/// cheap. It is the only state shared between threads.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    root: Node,
    leaves: Vec<String>,
}

impl Snapshot {
    pub(crate) fn new(root: Node) -> Self {
        let mut leaves = Vec::new();
        collect(&root, &mut Vec::new(), &mut leaves);
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
        self.leaves.iter().map(String::as_str)
    }

    /// Leaf keys and values in lexicographic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.leaves
            .iter()
            .filter_map(|key| self.get(key).map(|value| (key.as_str(), value)))
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

fn collect(node: &Node, segments: &mut Vec<Segment>, out: &mut Vec<String>) {
    match &node.value {
        Value::Map(entries) => {
            for (key, child) in entries {
                segments.push(Segment::Key(key.clone()));
                collect(child, segments, out);
                segments.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter().enumerate() {
                segments.push(Segment::Index(index));
                collect(child, segments, out);
                segments.pop();
            }
        }
        _ => out.push(render_path(segments)),
    }
}

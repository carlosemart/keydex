//! Structural merge and path insertion.

use std::collections::BTreeMap;

use crate::source::Segment;
use crate::value::{Node, Provenance, Value};

/// Merges `high` over `low`.
///
/// Maps merge key by key; sequences and scalars are replaced whole. That is the rule the
/// precedence model describes, and it is only expressible because the value is a tree.
#[must_use]
pub fn merge(low: Node, high: Node) -> Node {
    let Node {
        value: low_value,
        span: low_span,
        provenance: low_provenance,
        shadowed: mut low_shadowed,
        ..
    } = low;
    let Node {
        value: high_value,
        span: high_span,
        provenance: high_provenance,
        shadowed: high_shadowed,
        refs: high_refs,
    } = high;

    match (low_value, high_value) {
        (Value::Map(mut left), Value::Map(right)) => {
            for (key, value) in right {
                let merged = match left.remove(&key) {
                    Some(existing) => merge(existing, value),
                    None => value,
                };
                left.insert(key, merged);
            }
            Node {
                value: Value::Map(left),
                span: high_span.or(low_span),
                provenance: high_provenance.or(low_provenance),
                shadowed: {
                    low_shadowed.extend(high_shadowed);
                    low_shadowed
                },
                refs: high_refs,
            }
        }
        (_, value) => {
            let mut shadowed = high_shadowed;
            shadowed.extend(low_shadowed);
            if let Some(previous) = low_provenance {
                shadowed.push(previous);
            }
            Node {
                value,
                span: high_span,
                provenance: high_provenance,
                shadowed,
                refs: high_refs,
            }
        }
    }
}

/// Inserts a leaf at `segments`, creating the intermediate maps and sequences.
#[must_use]
pub fn insert(node: Node, segments: &[Segment], value: Value, provenance: Provenance) -> Node {
    let Node {
        value: current,
        span,
        provenance: existing,
        shadowed,
        refs,
    } = node;

    let Some((head, rest)) = segments.split_first() else {
        return Node {
            value,
            span: provenance.span.clone().or(span),
            provenance: Some(provenance),
            shadowed,
            refs,
        };
    };

    match head {
        Segment::Key(key) => {
            let mut map = match current {
                Value::Map(map) => map,
                _ => BTreeMap::new(),
            };
            let child = map.remove(key).unwrap_or_else(Node::empty_map);
            map.insert(key.clone(), insert(child, rest, value, provenance));
            Node {
                value: Value::Map(map),
                span,
                provenance: existing,
                shadowed,
                refs,
            }
        }
        Segment::Index(index) => {
            let mut items = match current {
                Value::Seq(items) => items,
                _ => Vec::new(),
            };
            while items.len() <= *index {
                items.push(Node::empty_map());
            }
            let child = std::mem::take(&mut items[*index]);
            items[*index] = insert(child, rest, value, provenance);
            Node {
                value: Value::Seq(items),
                span,
                provenance: existing,
                shadowed,
                refs,
            }
        }
    }
}

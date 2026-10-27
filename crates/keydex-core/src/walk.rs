//! Tree traversal shared by every stage.
//!
//! Before this existed, six modules walked the tree with their own recursion; one of them
//! inevitably forgot a case. Every walk now goes through these two functions.

use crate::source::Segment;
use crate::value::{Node, Value};

/// Visits every node, parents before children.
pub(crate) fn visit<F>(node: &Node, path: &mut Vec<Segment>, f: &mut F)
where
    F: FnMut(&[Segment], &Node),
{
    f(path, node);
    match &node.value {
        Value::Map(entries) => {
            for (key, child) in entries {
                path.push(Segment::Key(key.clone()));
                visit(child, path, f);
                path.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter().enumerate() {
                path.push(Segment::Index(index));
                visit(child, path, f);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Mutable variant of [`visit`].
pub(crate) fn visit_mut<F>(node: &mut Node, path: &mut Vec<Segment>, f: &mut F)
where
    F: FnMut(&[Segment], &mut Node),
{
    f(path, node);
    match &mut node.value {
        Value::Map(entries) => {
            for (key, child) in entries.iter_mut() {
                path.push(Segment::Key(key.clone()));
                visit_mut(child, path, f);
                path.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter_mut().enumerate() {
                path.push(Segment::Index(index));
                visit_mut(child, path, f);
                path.pop();
            }
        }
        _ => {}
    }
}

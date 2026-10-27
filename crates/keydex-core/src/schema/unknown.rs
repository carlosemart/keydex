//! Keys the schema does not use, with edit-distance suggestions.

use crate::source::Path;
use crate::value::{Node, Span, Value};

use super::Schema;
use super::{Field, FieldType};
use crate::snapshot::Snapshot;

/// Key present in the configuration that the schema does not use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey {
    /// Path as it appears in the snapshot.
    pub path: String,
    /// Closest schema path, when there is one reasonably near.
    pub suggestion: Option<String>,
    /// Position in the document, when known.
    pub span: Option<Span>,
}

/// Returns the snapshot keys that no schema path recognises.
#[must_use]
pub fn unknown_keys(snapshot: &Snapshot, schema: &Schema) -> Vec<UnknownKey> {
    let mut known = Vec::new();
    collect_leaf_paths(&schema.root, "", &mut known);

    let mut out = Vec::new();
    walk_unknown(
        &schema.root,
        Some(snapshot.root()),
        &Path::root(),
        &known,
        &mut out,
    );
    out
}

fn walk_unknown(
    field: &Field,
    node: Option<&Node>,
    path: &Path,
    known: &[String],
    out: &mut Vec<UnknownKey>,
) {
    let Some(node) = node else {
        return;
    };

    match &field.kind {
        FieldType::Struct { fields } => {
            let Value::Map(entries) = &node.value else {
                return;
            };
            for (key, child) in entries {
                let child_path = path.clone().with_key(key.as_str());
                match fields.get(key) {
                    Some(child_field) => {
                        walk_unknown(child_field, Some(child), &child_path, known, out);
                    }
                    None => report_unknown(child, &child_path, known, out),
                }
            }
        }
        FieldType::List { items } => {
            let Value::Seq(entries) = &node.value else {
                return;
            };
            for (index, child) in entries.iter().enumerate() {
                walk_unknown(
                    items,
                    Some(child),
                    &path.clone().with_index(index),
                    known,
                    out,
                );
            }
        }
        FieldType::Optional { inner } | FieldType::Secret { inner } => {
            walk_unknown(inner, Some(node), path, known, out);
        }
        _ => {}
    }
}

/// Reports an unused subtree: leaves get their own entry so the suggestion is precise.
fn report_unknown(node: &Node, path: &Path, known: &[String], out: &mut Vec<UnknownKey>) {
    match &node.value {
        Value::Map(entries) if !entries.is_empty() => {
            for (key, child) in entries {
                report_unknown(child, &path.clone().with_key(key.as_str()), known, out);
            }
        }
        Value::Seq(items) if !items.is_empty() => {
            for (index, child) in items.iter().enumerate() {
                report_unknown(child, &path.clone().with_index(index), known, out);
            }
        }
        _ => {
            let rendered = path.to_string();
            out.push(UnknownKey {
                suggestion: best_suggestion(&rendered, known),
                path: rendered,
                span: node.effective_span().cloned(),
            });
        }
    }
}

fn collect_leaf_paths(node: &Field, path: &str, out: &mut Vec<String>) {
    match &node.kind {
        FieldType::Struct { fields } => {
            for (name, child) in fields {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                collect_leaf_paths(child, &child_path, out);
            }
        }
        FieldType::List { items } => collect_leaf_paths(items, path, out),
        FieldType::Map { values } => collect_leaf_paths(values, path, out),
        FieldType::Optional { inner } | FieldType::Secret { inner } => {
            collect_leaf_paths(inner, path, out);
        }
        _ => {
            if !path.is_empty() {
                out.push(path.to_string());
            }
        }
    }
}

fn best_suggestion(unknown: &str, known: &[String]) -> Option<String> {
    let unknown_last = last_segment(unknown);
    let mut best: Option<(usize, &String)> = None;

    for candidate in known {
        let full = levenshtein(unknown, candidate);
        let last = levenshtein(unknown_last, last_segment(candidate)).saturating_add(1);
        let distance = full.min(last);
        if best.is_none_or(|(current, _)| distance < current) {
            best = Some((distance, candidate));
        }
    }

    match best {
        Some((distance, candidate)) if distance <= 2 => Some(candidate.clone()),
        _ => None,
    }
}

fn last_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];

    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let deletion = previous[j + 1] + 1;
            let insertion = current[j] + 1;
            let substitution = previous[j] + cost;
            current[j + 1] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b.len()]
}

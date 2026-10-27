//! Relaxed binding of environment variables to schema paths.

use std::collections::BTreeMap;

use crate::error::{Error, Result};

use super::{Field, FieldType};

/// Matches environment variables against schema paths (relaxed binding).
pub fn bind_env(
    root: &Field,
    env: &BTreeMap<String, String>,
    prefix: &str,
) -> Result<Vec<(String, String)>> {
    let mut dotted: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut relaxed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut dynamic: Vec<String> = Vec::new();
    collect_paths(root, "", &mut dotted, &mut relaxed, &mut dynamic);

    let wanted = prefix.to_ascii_uppercase();
    let mut bindings = Vec::new();

    for (name, value) in env {
        let upper = name.to_ascii_uppercase();
        let Some(rest) = upper.strip_prefix(&wanted) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let relaxed_key = rest.to_ascii_lowercase();
        let dotted_key = relaxed_key.replace("__", ".");

        let candidates = dotted
            .get(&dotted_key)
            .or_else(|| relaxed.get(&relaxed_key));

        let Some(paths) = candidates else {
            // A free-form map has no static paths: any key under its prefix binds to it.
            let prefix = dynamic
                .iter()
                .filter(|prefix| dotted_key.starts_with(&format!("{prefix}.")))
                .max_by_key(|prefix| prefix.len());
            if let Some(_prefix) = prefix {
                bindings.push((dotted_key.clone(), value.clone()));
            }
            continue;
        };
        match paths.as_slice() {
            [only] => bindings.push((only.clone(), value.clone())),
            _ => {
                return Err(Error::AmbiguousBinding {
                    env_var: name.clone(),
                    candidates: paths.clone(),
                });
            }
        }
    }

    bindings.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(bindings)
}

fn collect_paths(
    node: &Field,
    path: &str,
    dotted: &mut BTreeMap<String, Vec<String>>,
    relaxed: &mut BTreeMap<String, Vec<String>>,
    dynamic: &mut Vec<String>,
) {
    match &node.kind {
        FieldType::Struct { fields } => {
            for (name, child) in fields {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                collect_paths(child, &child_path, dotted, relaxed, dynamic);
            }
        }
        // Lists are not mapped from the environment in this version.
        FieldType::List { .. } => {}
        FieldType::Map { .. } => {
            // The keys inside a map are not known statically: only the prefix is.
            if !path.is_empty() {
                dynamic.push(path.to_string());
            }
        }
        FieldType::Optional { inner } | FieldType::Secret { inner } => {
            collect_paths(inner, path, dotted, relaxed, dynamic);
        }
        _ => {
            if !path.is_empty() {
                dotted
                    .entry(path.to_string())
                    .or_default()
                    .push(path.to_string());
                relaxed
                    .entry(path.replace('.', "_"))
                    .or_default()
                    .push(path.to_string());
            }
        }
    }
}

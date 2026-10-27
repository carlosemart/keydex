//! Environment variable source.

use std::collections::BTreeMap;

use crate::source::RawEntry;
use crate::value::Value;

/// Turns already captured environment variables into flat entries.
///
/// **Dictionary mode** rules (no schema, see specification §6.2): the prefix is stripped
/// case-insensitively, `__` separates levels and `_` is a literal character. No guessing:
/// without a schema we do not infer.
#[must_use]
pub fn parse(env: &BTreeMap<String, String>, prefix: &str) -> Vec<RawEntry> {
    let wanted = prefix.to_ascii_uppercase();
    let mut out = Vec::new();

    for (name, value) in env {
        let upper = name.to_ascii_uppercase();
        let Some(rest) = upper.strip_prefix(&wanted) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        out.push(RawEntry {
            path: rest.to_ascii_lowercase().replace("__", "."),
            value: Value::str(value.as_str()),
            span: None,
        });
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

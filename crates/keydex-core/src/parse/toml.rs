//! TOML with spans, through `toml_edit`.
//!
//! Note: `DocumentMut` drops spans; `Document` (formerly `ImDocument`) must be used.

use toml_edit::{Document, Item, Table, Value as TomlValue};

use crate::error::{Error, Result};
use crate::parse::{LineIndex, join};
use crate::source::RawEntry;
use crate::value::{Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<RawEntry>> {
    let doc: Document<&str> = Document::parse(src).map_err(|err| Error::Parse {
        file: file.to_string(),
        message: err.to_string(),
    })?;

    let index = LineIndex::new(src);
    let mut out = Vec::new();
    walk_table(doc.as_table(), "", file, src, &index, &mut out);
    Ok(out)
}

fn walk_table(
    table: &Table,
    prefix: &str,
    file: &str,
    src: &str,
    index: &LineIndex,
    out: &mut Vec<RawEntry>,
) {
    for (key, item) in table.iter() {
        let path = join(prefix, key);
        match item {
            Item::Table(nested) => walk_table(nested, &path, file, src, index, out),
            Item::ArrayOfTables(tables) => {
                for (i, nested) in tables.iter().enumerate() {
                    walk_table(nested, &format!("{path}[{i}]"), file, src, index, out);
                }
            }
            Item::Value(value) => walk_value(value, &path, file, src, index, out),
            Item::None => {}
        }
    }
}

fn walk_value(
    value: &TomlValue,
    path: &str,
    file: &str,
    src: &str,
    index: &LineIndex,
    out: &mut Vec<RawEntry>,
) {
    match value {
        TomlValue::Array(array) => {
            for (i, item) in array.iter().enumerate() {
                walk_value(item, &format!("{path}[{i}]"), file, src, index, out);
            }
        }
        TomlValue::InlineTable(table) => {
            for (key, item) in table.iter() {
                walk_value(item, &format!("{path}.{key}"), file, src, index, out);
            }
        }
        leaf => {
            let span = leaf.span().map(|range| {
                let (line, col) = index.line_col(src, range.start);
                Span {
                    file: file.to_string(),
                    line,
                    col,
                }
            });
            out.push(RawEntry {
                path: path.to_string(),
                value: toml_value(leaf),
                span,
            });
        }
    }
}

fn toml_value(value: &TomlValue) -> Value {
    match value {
        TomlValue::String(s) => Value::str(s.value().as_str()),
        TomlValue::Integer(i) => Value::Int(*i.value()),
        TomlValue::Float(f) => Value::Float(*f.value()),
        TomlValue::Boolean(b) => Value::Bool(*b.value()),
        TomlValue::Datetime(d) => Value::str(d.value().to_string()),
        _ => Value::Null,
    }
}

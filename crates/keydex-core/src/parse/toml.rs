//! TOML with spans, through `toml_edit`.
//!
//! Note: `DocumentMut` drops spans; `Document` (formerly `ImDocument`) must be used.

use std::collections::BTreeMap;

use toml_edit::{Document, Item, Table, Value as TomlValue};

use crate::error::{Error, Result};
use crate::parse::LineIndex;
use crate::value::{Node, Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<Node>> {
    let doc: Document<&str> = Document::parse(src).map_err(|err| {
        // `TomlError`'s Display echoes the offending source line, which can hold a secret.
        // Keep the position and the parser's message, drop the excerpt.
        let index = LineIndex::new(src);
        let position = err.span().map_or_else(String::new, |span| {
            let (line, col) = index.line_col(src, span.start);
            format!(" at line {line}, column {}", col + 1)
        });
        Error::Parse {
            file: file.to_string(),
            message: format!("{}{position}", err.message()),
        }
    })?;

    let index = LineIndex::new(src);
    let root = table_to_node(doc.as_table(), file, src, &index);
    Ok(vec![root])
}

fn table_to_node(table: &Table, file: &str, src: &str, index: &LineIndex) -> Node {
    let mut entries = BTreeMap::new();
    for (key, item) in table.iter() {
        entries.insert(key.to_string(), item_to_node(item, file, src, index));
    }
    Node::new(Value::Map(entries), None)
}

fn item_to_node(item: &Item, file: &str, src: &str, index: &LineIndex) -> Node {
    match item {
        Item::Table(table) => table_to_node(table, file, src, index),
        Item::ArrayOfTables(tables) => Node::new(
            Value::Seq(
                tables
                    .iter()
                    .map(|table| table_to_node(table, file, src, index))
                    .collect(),
            ),
            None,
        ),
        Item::Value(value) => value_to_node(value, file, src, index),
        Item::None => Node::new(Value::Null, None),
    }
}

fn value_to_node(value: &TomlValue, file: &str, src: &str, index: &LineIndex) -> Node {
    let span = value.span().map(|range| {
        let (line, col) = index.line_col(src, range.start);
        Span {
            file: file.to_string(),
            line,
            col,
        }
    });

    let value = match value {
        TomlValue::Array(array) => Value::Seq(
            array
                .iter()
                .map(|item| value_to_node(item, file, src, index))
                .collect(),
        ),
        TomlValue::InlineTable(table) => {
            let mut entries = BTreeMap::new();
            for (key, item) in table.iter() {
                entries.insert(key.to_string(), value_to_node(item, file, src, index));
            }
            Value::Map(entries)
        }
        TomlValue::String(s) => Value::str(s.value().as_str()),
        TomlValue::Integer(i) => Value::Int(*i.value()),
        TomlValue::Float(f) => Value::Float(*f.value()),
        TomlValue::Boolean(b) => Value::Bool(*b.value()),
        TomlValue::Datetime(d) => Value::str(d.value().to_string()),
    };

    Node::new(value, span)
}

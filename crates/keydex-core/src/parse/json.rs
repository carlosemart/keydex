//! JSON with spans, through `jsonc-parser`.
//!
//! `jiter` was discarded in spike 0001: it is faster but exposes no positions.

use std::collections::BTreeMap;

use jsonc_parser::ast::Value as JsonValue;
use jsonc_parser::common::Ranged;

use crate::error::{Error, Result};
use crate::parse::LineIndex;
use crate::value::{Node, Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<Node>> {
    let parsed = jsonc_parser::parse_to_ast(src, &Default::default(), &Default::default())
        .map_err(|err| Error::Parse {
            file: file.to_string(),
            message: err.to_string(),
        })?;

    let index = LineIndex::new(src);
    let root = parsed
        .value
        .as_ref()
        .map(|value| to_node(value, file, src, &index))
        .unwrap_or_else(Node::empty_map);

    Ok(vec![root])
}

fn to_node(value: &JsonValue, file: &str, src: &str, index: &LineIndex) -> Node {
    let range = value.range();
    let (line, col) = index.line_col(src, range.start);
    let span = Some(Span {
        file: file.to_string(),
        line,
        col,
    });

    let value = match value {
        JsonValue::Object(object) => {
            let mut entries = BTreeMap::new();
            for property in &object.properties {
                entries.insert(
                    property.name.clone().into_string(),
                    to_node(&property.value, file, src, index),
                );
            }
            Value::Map(entries)
        }
        JsonValue::Array(array) => Value::Seq(
            array
                .elements
                .iter()
                .map(|item| to_node(item, file, src, index))
                .collect(),
        ),
        leaf => json_scalar(leaf),
    };

    Node::new(value, span)
}

fn json_scalar(value: &JsonValue) -> Value {
    match value {
        JsonValue::StringLit(s) => Value::str(s.value.to_string()),
        JsonValue::BooleanLit(b) => Value::Bool(b.value),
        JsonValue::NullKeyword(_) => Value::Null,
        JsonValue::NumberLit(n) => {
            // `jsonc-parser` keeps the original literal as text: `5432` does not become
            // `5432.0`.
            match n.value.parse::<i64>() {
                Ok(i) => Value::Int(i),
                Err(_) => match n.value.parse::<f64>() {
                    Ok(f) => Value::Float(f),
                    Err(_) => Value::str(n.value.to_string()),
                },
            }
        }
        _ => Value::Null,
    }
}

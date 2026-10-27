//! JSON con spans, vía `jsonc-parser`.
//!
//! `jiter` se descartó en el spike 0001: es más rápido pero no expone posiciones.

use jsonc_parser::ast::Value as JsonValue;
use jsonc_parser::common::Ranged;

use crate::error::{Error, Result};
use crate::parse::{LineIndex, join};
use crate::source::RawEntry;
use crate::value::{Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<RawEntry>> {
    let parsed = jsonc_parser::parse_to_ast(src, &Default::default(), &Default::default())
        .map_err(|err| Error::Parse {
            file: file.to_string(),
            message: format!("{err:?}"),
        })?;

    let index = LineIndex::new(src);
    let mut out = Vec::new();
    if let Some(root) = parsed.value.as_ref() {
        walk(root, "", file, src, &index, &mut out);
    }
    Ok(out)
}

fn walk(
    value: &JsonValue,
    path: &str,
    file: &str,
    src: &str,
    index: &LineIndex,
    out: &mut Vec<RawEntry>,
) {
    match value {
        JsonValue::Object(object) => {
            for property in &object.properties {
                let child = join(path, &property.name.clone().into_string());
                walk(&property.value, &child, file, src, index, out);
            }
        }
        JsonValue::Array(array) => {
            for (i, item) in array.elements.iter().enumerate() {
                walk(item, &format!("{path}[{i}]"), file, src, index, out);
            }
        }
        leaf => {
            if path.is_empty() {
                return;
            }
            let range = leaf.range();
            let (line, col) = index.line_col(src, range.start);
            out.push(RawEntry {
                path: path.to_string(),
                value: json_value(leaf),
                span: Some(Span {
                    file: file.to_string(),
                    line,
                    col,
                }),
            });
        }
    }
}

fn json_value(value: &JsonValue) -> Value {
    match value {
        JsonValue::StringLit(s) => Value::str(s.value.to_string()),
        JsonValue::BooleanLit(b) => Value::Bool(b.value),
        JsonValue::NullKeyword(_) => Value::Null,
        JsonValue::NumberLit(n) => {
            // `jsonc-parser` conserva el literal original como texto: no convertimos
            // `5432` en `5432.0`.
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

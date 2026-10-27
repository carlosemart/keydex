//! YAML with spans, through `saphyr` (which bundles `marked_yaml`).

use saphyr::{LoadableYamlNode, MarkedYaml, Scalar, YamlData};

use crate::error::{Error, Result};
use crate::parse::join;
use crate::source::RawEntry;
use crate::value::{Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<RawEntry>> {
    let docs = MarkedYaml::load_from_str(src).map_err(|err| Error::Parse {
        file: file.to_string(),
        message: format!("{err:?}"),
    })?;

    let mut out = Vec::new();
    for doc in &docs {
        visit(doc, "", file, &mut out);
    }
    Ok(out)
}

fn visit(node: &MarkedYaml<'_>, path: &str, file: &str, out: &mut Vec<RawEntry>) {
    match &node.data {
        YamlData::Mapping(mapping) => {
            for (key, value) in mapping.iter() {
                let child = join(path, &key_of(&key.data));
                visit(value, &child, file, out);
            }
        }
        YamlData::Sequence(sequence) => {
            for (i, value) in sequence.iter().enumerate() {
                visit(value, &format!("{path}[{i}]"), file, out);
            }
        }
        data => {
            if path.is_empty() {
                return;
            }
            out.push(RawEntry {
                path: path.to_string(),
                value: scalar_value(data),
                span: Some(Span {
                    file: file.to_string(),
                    line: u32::try_from(node.span.start.line()).unwrap_or(u32::MAX),
                    col: u32::try_from(node.span.start.col()).unwrap_or(u32::MAX),
                }),
            });
        }
    }
}

fn key_of<'a>(data: &YamlData<'a, MarkedYaml<'a>>) -> String {
    match data {
        YamlData::Value(Scalar::String(s)) => s.to_string(),
        YamlData::Value(Scalar::Integer(i)) => i.to_string(),
        YamlData::Value(Scalar::Boolean(b)) => b.to_string(),
        YamlData::Representation(s, _, _) => s.to_string(),
        _ => String::from("?"),
    }
}

fn scalar_value<'a>(data: &YamlData<'a, MarkedYaml<'a>>) -> Value {
    match data {
        YamlData::Value(scalar) => scalar_of(scalar),
        YamlData::Representation(s, _, _) => Value::str(s.as_ref()),
        _ => Value::Null,
    }
}

fn scalar_of(scalar: &Scalar<'_>) -> Value {
    match scalar {
        Scalar::Null => Value::Null,
        Scalar::Boolean(b) => Value::Bool(*b),
        Scalar::Integer(i) => Value::Int(*i),
        Scalar::FloatingPoint(f) => Value::Float(f.0),
        Scalar::String(s) => Value::str(s.as_ref()),
    }
}

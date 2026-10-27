//! YAML with spans, through `saphyr` (which bundles `marked_yaml`).

use std::collections::BTreeMap;

use saphyr::{LoadableYamlNode, MarkedYaml, Scalar, YamlData};

use crate::error::{Error, Result};
use crate::value::{Node, Span, Value};

pub(super) fn parse(src: &str, file: &str) -> Result<Vec<Node>> {
    let docs = MarkedYaml::load_from_str(src).map_err(|err| Error::Parse {
        file: file.to_string(),
        message: format!("{err:?}"),
    })?;

    Ok(docs.iter().map(|doc| to_node(doc, file)).collect())
}

fn to_node(node: &MarkedYaml<'_>, file: &str) -> Node {
    let span = Some(Span {
        file: file.to_string(),
        line: u32::try_from(node.span.start.line()).unwrap_or(u32::MAX),
        col: u32::try_from(node.span.start.col()).unwrap_or(u32::MAX),
    });

    let value = match &node.data {
        YamlData::Mapping(mapping) => {
            let mut entries = BTreeMap::new();
            for (key, value) in mapping.iter() {
                entries.insert(key_of(&key.data), to_node(value, file));
            }
            Value::Map(entries)
        }
        YamlData::Sequence(sequence) => {
            Value::Seq(sequence.iter().map(|item| to_node(item, file)).collect())
        }
        data => scalar_value(data),
    };

    Node::new(value, span)
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

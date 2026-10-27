//! Coercion of the snapshot into a typed tree, with aggregated errors.

use serde_json::{Map, Value as Json};

use super::Schema;
use crate::snapshot::Snapshot;
use crate::source::Path;
use crate::value::{Node, Value};

use super::{Field, FieldType, ValidationError};

/// Result of the coercion: the typed tree or the list of errors.
pub type Coerced = core::result::Result<Json, Vec<ValidationError>>;

/// Coerces and validates the snapshot against the schema.
pub fn coerce(snapshot: &Snapshot, schema: &Schema) -> Coerced {
    let mut errors = Vec::new();
    let slot = node(
        &schema.root,
        Some(snapshot.root()),
        &Path::root(),
        &mut errors,
    );

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(match slot {
        Slot::Value(value) => value,
        Slot::Missing => Json::Null,
    })
}

enum Slot {
    Missing,
    Value(Json),
}

impl Slot {
    fn into_option(self) -> Option<Json> {
        match self {
            Self::Missing => None,
            Self::Value(value) => Some(value),
        }
    }
}

fn node(
    field: &Field,
    current: Option<&Node>,
    path: &Path,
    errors: &mut Vec<ValidationError>,
) -> Slot {
    match &field.kind {
        FieldType::Struct { fields } => {
            if current.is_none() && field.has_default {
                return Slot::Missing;
            }
            let map = match current.map(|node| &node.value) {
                Some(Value::Map(map)) => Some(map),
                Some(other) => {
                    errors.push(error(
                        path,
                        current,
                        format!("expected an object, got {}", kind_of(other)),
                    ));
                    return Slot::Missing;
                }
                None => None,
            };

            let mut object = Map::new();
            for (name, child) in fields {
                let child_node = map.and_then(|entries| entries.get(name));
                let child_path = path.clone().with_key(name.as_str());
                if let Some(value) = node(child, child_node, &child_path, errors).into_option() {
                    object.insert(name.clone(), value);
                }
            }

            if object.is_empty() && !field.required {
                Slot::Missing
            } else {
                Slot::Value(Json::Object(object))
            }
        }
        FieldType::Optional { inner } => match current.map(|node| &node.value) {
            // Absent: let the caller apply its default; otherwise the value is `None`.
            None if field.has_default => Slot::Missing,
            None | Some(Value::Null) => Slot::Value(Json::Null),
            _ => node(inner, current, path, errors),
        },
        FieldType::Secret { inner } => match current {
            None if field.has_default => Slot::Missing,
            None if field.required => {
                errors.push(ValidationError {
                    path: path.to_string(),
                    message: String::from("required field missing"),
                    span: None,
                });
                Slot::Missing
            }
            _ => node(inner, current, path, errors),
        },
        FieldType::Map { values } => match current.map(|node| &node.value) {
            Some(Value::Map(entries)) => {
                let mut object = Map::new();
                for (key, child) in entries {
                    let child_path = path.clone().with_key(key.as_str());
                    if let Some(value) =
                        node(values, Some(child), &child_path, errors).into_option()
                    {
                        object.insert(key.clone(), value);
                    }
                }
                Slot::Value(Json::Object(object))
            }
            Some(other) => {
                errors.push(error(
                    path,
                    current,
                    format!("expected an object, got {}", kind_of(other)),
                ));
                Slot::Missing
            }
            None => {
                if field.required {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: String::from("required map missing"),
                        span: None,
                    });
                }
                Slot::Missing
            }
        },
        FieldType::List { items } => match current.map(|node| &node.value) {
            Some(Value::Seq(entries)) => {
                let mut values = Vec::with_capacity(entries.len());
                for (index, child) in entries.iter().enumerate() {
                    let child_path = path.clone().with_index(index);
                    let slot = node(items, Some(child), &child_path, errors);
                    values.push(slot.into_option().unwrap_or(Json::Null));
                }
                Slot::Value(Json::Array(values))
            }
            Some(other) => {
                errors.push(error(
                    path,
                    current,
                    format!("expected a list, got {}", kind_of(other)),
                ));
                Slot::Missing
            }
            None => {
                if field.required {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: String::from("required list missing"),
                        span: None,
                    });
                }
                Slot::Missing
            }
        },
        _ => scalar(field, current, path, errors),
    }
}

fn error(path: &Path, node: Option<&Node>, message: String) -> ValidationError {
    // A failing secret must not end up in a log or a traceback.
    let message = match node {
        Some(node) if node.sensitive => match message.split_once(" got ") {
            Some((head, _)) => format!("{head} got <redacted>"),
            None => message,
        },
        _ => message,
    };

    ValidationError {
        path: path.to_string(),
        message,
        span: node.and_then(|node| node.effective_span().cloned()),
    }
}

fn scalar(
    field: &Field,
    current: Option<&Node>,
    path: &Path,
    errors: &mut Vec<ValidationError>,
) -> Slot {
    let Some(node) = current else {
        if field.required {
            errors.push(ValidationError {
                path: path.to_string(),
                message: String::from("required field missing"),
                span: None,
            });
        }
        return Slot::Missing;
    };

    if node.value == Value::Null {
        if !matches!(field.kind, FieldType::Any) {
            if field.required {
                errors.push(error(
                    path,
                    Some(node),
                    String::from("required field is null"),
                ));
            }
            return Slot::Missing;
        }
        return Slot::Value(Json::Null);
    }

    match convert(&field.kind, &node.value) {
        Ok(json) => Slot::Value(json),
        Err(message) => {
            errors.push(error(path, Some(node), message));
            Slot::Missing
        }
    }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Int(_) | Value::Float(_) => "a number",
        Value::Str(_) => "a string",
        Value::Seq(_) => "a list",
        Value::Map(_) => "an object",
    }
}

fn convert(kind: &FieldType, value: &Value) -> core::result::Result<Json, String> {
    match kind {
        FieldType::Any => match value {
            Value::Null => Ok(Json::Null),
            Value::Bool(b) => Ok(Json::Bool(*b)),
            Value::Int(i) => Ok(Json::from(*i)),
            Value::Float(f) => Ok(Json::from(*f)),
            Value::Str(s) => Ok(Json::String(s.clone())),
            other => Err(format!("any does not accept {}", kind_of(other))),
        },
        FieldType::Str => match value {
            // Only text is text: a list stringified to "[a, b]" or a float to "1.1" is
            // silent corruption, not a conversion.
            Value::Str(s) => Ok(Json::String(s.clone())),
            other => Err(format!("expected a string, got {}", kind_of(other))),
        },
        FieldType::Int => match value {
            Value::Int(i) => Ok(Json::from(*i)),
            Value::Str(text) => text
                .parse::<i64>()
                .map(Json::from)
                .map_err(|_| format!("expected an integer, got {text:?}")),
            other => Err(format!("expected an integer, got {}", kind_of(other))),
        },
        FieldType::Float => match value {
            Value::Float(f) => Ok(Json::from(*f)),
            Value::Int(i) => {
                Ok(serde_json::Number::from_f64(*i as f64).map_or(Json::Null, Json::Number))
            }
            Value::Str(text) => text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Json::Number)
                .ok_or_else(|| format!("expected a number, got {text:?}")),
            other => Err(format!("expected a number, got {}", kind_of(other))),
        },
        FieldType::Bool => match value {
            Value::Bool(b) => Ok(Json::Bool(*b)),
            Value::Str(text) => parse_bool(text)
                .map(Json::Bool)
                .ok_or_else(|| format!("expected a boolean, got {text:?}")),
            other => Err(format!("expected a boolean, got {}", kind_of(other))),
        },
        FieldType::Path => match value {
            Value::Str(s) if !s.is_empty() => Ok(Json::String(s.clone())),
            Value::Str(_) => Err(String::from("expected a non-empty path")),
            other => Err(format!("expected a path, got {}", kind_of(other))),
        },
        FieldType::Decimal => match value {
            Value::Str(s) if is_decimal(s) => Ok(Json::String(s.clone())),
            Value::Str(s) => Err(format!("expected a decimal number, got {s:?}")),
            other => Err(format!(
                "expected a decimal as text (to keep precision), got {}",
                kind_of(other)
            )),
        },
        FieldType::Uuid => match value {
            Value::Str(s) if is_uuid(s) => Ok(Json::String(s.to_ascii_lowercase())),
            Value::Str(s) => Err(format!("expected a UUID, got {s:?}")),
            other => Err(format!("expected a UUID, got {}", kind_of(other))),
        },
        FieldType::Enum { variants } => {
            let text = value.to_string();
            variants
                .iter()
                .find(|variant| variant.eq_ignore_ascii_case(&text))
                .cloned()
                .map(Json::String)
                .ok_or_else(|| {
                    format!(
                        "expected one of {}, got {text:?}",
                        variants
                            .iter()
                            .map(|v| format!("{v:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
        }
        other => Err(format!("unknown schema type: {other:?}")),
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn is_decimal(text: &str) -> bool {
    let body = text.strip_prefix(['-', '+']).unwrap_or(text);
    let mut parts = body.split('.');
    let (Some(integer), fraction) = (parts.next(), parts.next()) else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let integer_ok = !integer.is_empty() && integer.chars().all(|c| c.is_ascii_digit());
    let fraction_ok =
        fraction.is_none_or(|f| !f.is_empty() && f.chars().all(|c| c.is_ascii_digit()));
    integer_ok && fraction_ok
}

fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (i, byte) in bytes.iter().enumerate() {
        let expected_hyphen = matches!(i, 8 | 13 | 18 | 23);
        if expected_hyphen {
            if *byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

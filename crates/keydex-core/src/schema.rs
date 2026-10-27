//! Schema IR v1: descripción del modelo, binding relajado y coerción tipada.
//!
//! El IR lo genera la capa de Python a partir de dataclasses y lo consume el motor.
//! Es el contrato de compatibilidad entre ambos mundos: se versiona aparte del paquete
//! (ver ADR 0004).

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Map, Value as Json};

use crate::error::{Error, Result};
use crate::snapshot::Snapshot;
use crate::value::Value;

/// Versión del IR que entiende este motor.
pub const IR_VERSION: &str = "keydex.schema/v1";

/// Esquema de un modelo.
#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    /// Versión del contrato (`keydex.schema/v1`).
    pub ir: String,
    /// Nodo raíz, normalmente un `struct`.
    pub root: Field,
}

impl Schema {
    /// Parsea y valida un IR en JSON.
    pub fn from_json(text: &str) -> Result<Self> {
        let schema: Self = serde_json::from_str(text).map_err(|err| Error::Parse {
            file: String::from("<schema>"),
            message: err.to_string(),
        })?;
        if schema.ir != IR_VERSION {
            return Err(Error::UnsupportedFormat { format: schema.ir });
        }
        Ok(schema)
    }
}

/// Nodo del esquema.
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    /// Tipo: `str`, `int`, `float`, `bool`, `any`, `struct`, `optional` o `list`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `true` si el valor tiene que existir.
    #[serde(default)]
    pub required: bool,
    /// Default declarado en el modelo (informativo; el motor no lo aplica).
    #[serde(default)]
    pub default: Option<Json>,
    /// Campos del `struct`.
    #[serde(default)]
    pub fields: BTreeMap<String, Field>,
    /// Elemento de una `list`.
    #[serde(default)]
    pub items: Option<Box<Field>>,
    /// Tipo interno de un `optional`.
    #[serde(default)]
    pub inner: Option<Box<Field>>,
    /// Convertidor que debe aplicar la capa de Python (`enum`, `path`, `decimal`, `uuid`).
    #[serde(default)]
    pub converter: Option<String>,
}

/// Error de validación de un valor concreto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Ruta de la clave: `db.port`, `servers[0].host`.
    pub path: String,
    /// Qué está mal.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Clave presente en la configuración que el esquema no usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey {
    /// Ruta tal y como aparece en el snapshot.
    pub path: String,
    /// Ruta del esquema más parecida, si hay una razonablemente cerca.
    pub suggestion: Option<String>,
}

/// Devuelve las claves del snapshot que ninguna ruta del esquema reconoce.
///
/// Es la red de seguridad contra el fallo más silencioso de una migración: un override
/// que se escribe mal y no llega a aplicarse nunca.
#[must_use]
pub fn unknown_keys(snapshot: &Snapshot, schema: &Schema) -> Vec<UnknownKey> {
    let mut known = Vec::new();
    collect_leaf_paths(&schema.root, "", &mut known);

    snapshot
        .keys()
        .filter(|key| !is_known(&schema.root, key))
        .map(|key| UnknownKey {
            path: key.to_string(),
            suggestion: best_suggestion(key, &known),
        })
        .collect()
}

fn collect_leaf_paths(node: &Field, path: &str, out: &mut Vec<String>) {
    match node.kind.as_str() {
        "struct" => {
            for (name, child) in &node.fields {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                collect_leaf_paths(child, &child_path, out);
            }
        }
        "list" => {
            if let Some(items) = node.items.as_deref() {
                collect_leaf_paths(items, path, out);
            }
        }
        "optional" => {
            if let Some(inner) = node.inner.as_deref() {
                collect_leaf_paths(inner, path, out);
            }
        }
        _ => {
            if !path.is_empty() {
                out.push(path.to_string());
            }
        }
    }
}

fn is_known(root: &Field, path: &str) -> bool {
    let mut current = root;
    for segment in segments(path) {
        let node = unwrap_optional(current);
        if segment.starts_with('[') {
            if node.kind != "list" {
                return false;
            }
            match node.items.as_deref() {
                Some(items) => current = items,
                None => return false,
            }
        } else {
            if node.kind != "struct" {
                return false;
            }
            match node.fields.get(segment) {
                Some(field) => current = field,
                None => return false,
            }
        }
    }
    true
}

/// Descompone `servers[0].host` en `["servers", "[0]", "host"]`.
fn segments(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for part in path.split('.') {
        let Some(open) = part.find('[') else {
            out.push(part);
            continue;
        };
        if open > 0 {
            out.push(&part[..open]);
        }
        let mut rest = &part[open..];
        while let Some(close) = rest.find(']') {
            out.push(&rest[..=close]);
            rest = &rest[close + 1..];
            if rest.is_empty() {
                break;
            }
        }
    }
    out
}

fn unwrap_optional(mut node: &Field) -> &Field {
    while node.kind == "optional" {
        match node.inner.as_deref() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn best_suggestion(unknown: &str, known: &[String]) -> Option<String> {
    let unknown_last = last_segment(unknown);
    let mut best: Option<(usize, &String)> = None;

    for candidate in known {
        let full = levenshtein(unknown, candidate);
        // Comparar también el último segmento permite sugerir `name` cuando el usuario
        // anidó mal (`app.name`).
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
/// Resultado de la coerción: el árbol tipado o la lista de errores.
pub type Coerced = core::result::Result<Json, Vec<ValidationError>>;

/// Coacciona y valida el snapshot contra el esquema.
///
/// Devuelve un árbol JSON con los valores ya tipados. Los campos ausentes que no son
/// obligatorios se omiten, para que Python aplique los defaults del dataclass.
pub fn coerce(snapshot: &Snapshot, schema: &Schema) -> Coerced {
    let mut errors = Vec::new();
    let slot = node(&schema.root, "", snapshot, &mut errors);

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

fn node(field: &Field, path: &str, snapshot: &Snapshot, errors: &mut Vec<ValidationError>) -> Slot {
    match field.kind.as_str() {
        "struct" => {
            let mut object = Map::new();
            for (name, child) in &field.fields {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                if let Some(value) = node(child, &child_path, snapshot, errors).into_option() {
                    object.insert(name.clone(), value);
                }
            }
            if object.is_empty() && !field.required {
                Slot::Missing
            } else {
                Slot::Value(Json::Object(object))
            }
        }
        "optional" => {
            if snapshot.get(path).is_none() {
                return Slot::Value(Json::Null);
            }
            match snapshot.get(path) {
                Some(Value::Null) => Slot::Value(Json::Null),
                _ => match field.inner.as_deref() {
                    Some(inner) => node(inner, path, snapshot, errors),
                    None => {
                        errors.push(ValidationError {
                            path: path.to_string(),
                            message: String::from("optional sin tipo interno"),
                        });
                        Slot::Missing
                    }
                },
            }
        }
        "list" => list(field, path, snapshot, errors),
        _ => scalar(field, path, snapshot, errors),
    }
}

fn list(field: &Field, path: &str, snapshot: &Snapshot, errors: &mut Vec<ValidationError>) -> Slot {
    let Some(item) = field.items.as_deref() else {
        errors.push(ValidationError {
            path: path.to_string(),
            message: String::from("list sin tipo de elemento"),
        });
        return Slot::Missing;
    };

    let prefix = format!("{path}[");
    let mut indexed: Vec<(usize, String)> = Vec::new();
    for key in snapshot.keys_with_prefix(&prefix) {
        let Some(index) = key
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(|digits| digits.parse::<usize>().ok())
        else {
            continue;
        };
        indexed.push((index, key.to_string()));
    }

    if indexed.is_empty() {
        if field.required {
            errors.push(ValidationError {
                path: path.to_string(),
                message: String::from("lista requerida ausente"),
            });
        }
        return Slot::Missing;
    }

    indexed.sort_by_key(|(index, _)| *index);
    let mut values = Vec::with_capacity(indexed.len());
    for (index, key) in indexed {
        if let Some(value) = node(item, &key, snapshot, errors).into_option() {
            values.push(value);
        } else {
            values.push(Json::Null);
            let _ = index;
        }
    }
    Slot::Value(Json::Array(values))
}

fn scalar(
    field: &Field,
    path: &str,
    snapshot: &Snapshot,
    errors: &mut Vec<ValidationError>,
) -> Slot {
    let Some(value) = snapshot.get(path) else {
        if field.required {
            errors.push(ValidationError {
                path: path.to_string(),
                message: String::from("campo requerido ausente"),
            });
        }
        return Slot::Missing;
    };

    match convert(field, value) {
        Ok(json) => Slot::Value(json),
        Err(message) => {
            errors.push(ValidationError {
                path: path.to_string(),
                message,
            });
            Slot::Missing
        }
    }
}

fn convert(field: &Field, value: &Value) -> core::result::Result<Json, String> {
    match field.kind.as_str() {
        "str" | "any" => Ok(Json::String(value.to_string())),
        "int" => match value {
            Value::Int(i) => Ok(Json::from(*i)),
            Value::Str(text) => text
                .parse::<i64>()
                .map(Json::from)
                .map_err(|_| format!("se esperaba un entero, llegó {text:?}")),
            other => Err(format!("se esperaba un entero, llegó {other:?}")),
        },
        "float" => match value {
            Value::Float(f) => Ok(Json::from(*f)),
            Value::Int(i) => {
                Ok(serde_json::Number::from_f64(*i as f64).map_or(Json::Null, Json::Number))
            }
            Value::Str(text) => text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Json::Number)
                .ok_or_else(|| format!("se esperaba un número, llegó {text:?}")),
            other => Err(format!("se esperaba un número, llegó {other:?}")),
        },
        "bool" => match value {
            Value::Bool(b) => Ok(Json::Bool(*b)),
            Value::Str(text) => parse_bool(text)
                .map(Json::Bool)
                .ok_or_else(|| format!("se esperaba un booleano, llegó {text:?}")),
            other => Err(format!("se esperaba un booleano, llegó {other:?}")),
        },
        other => Err(format!("tipo de esquema desconocido: {other}")),
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Empareja variables de entorno con rutas del esquema (binding relajado).
///
/// Con esquema no hay heurísticas: `APP_DB_HOST` casa con `db.host` porque la ruta
/// normalizada coincide, y `APP_DB__HOST` casa de forma explícita. La coincidencia
/// exacta con la ruta gana; solo si no existe se usa la forma relajada. Si dos rutas
/// del esquema empatan se devuelve [`Error::AmbiguousBinding`].
pub fn bind_env(
    root: &Field,
    env: &BTreeMap<String, String>,
    prefix: &str,
) -> Result<Vec<(String, String)>> {
    let mut dotted: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut relaxed: BTreeMap<String, Vec<String>> = BTreeMap::new();
    collect_paths(root, "", &mut dotted, &mut relaxed);

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
) {
    match node.kind.as_str() {
        "struct" => {
            for (name, child) in &node.fields {
                let child_path = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                collect_paths(child, &child_path, dotted, relaxed);
            }
        }
        // Las listas no se mapean por entorno en esta versión.
        "list" => {}
        "optional" => {
            if let Some(inner) = node.inner.as_deref() {
                collect_paths(inner, path, dotted, relaxed);
            }
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

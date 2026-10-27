//! Interpolación de referencias `${...}`.
//!
//! Semántica (especificación §8): solo referencias y resolvers puros, sin expresiones,
//! sin `eval`. Soporta `${clave}`, `${clave:-default}`, `${env:VAR}` y anidamiento en los
//! valores por defecto. La resolución es memoizada y detecta ciclos con traza completa.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::value::Value;

/// Origen de una referencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target<'a> {
    Key(&'a str),
    Env(&'a str),
}

/// Placeholder localizado dentro de una cadena.
#[derive(Debug, Clone)]
struct Placeholder<'a> {
    start: usize,
    end: usize,
    target: Target<'a>,
    default: Option<&'a str>,
}

/// Valores resueltos y, por clave, las referencias directas usadas.
pub(crate) type ResolvedMaps = (BTreeMap<String, Value>, BTreeMap<String, Vec<String>>);

/// Resuelve todas las referencias del mapa de valores crudos.
///
/// Devuelve los valores ya resueltos y, por clave, las referencias directas usadas
/// (para `explain`).
pub(crate) fn resolve_all(
    raw: &BTreeMap<String, Value>,
    env: &BTreeMap<String, String>,
) -> Result<ResolvedMaps> {
    let mut resolver = Resolver {
        raw,
        env,
        done: BTreeMap::new(),
        refs: BTreeMap::new(),
        stack: Vec::new(),
    };

    for path in raw.keys() {
        resolver.resolve(path)?;
    }

    Ok((resolver.done, resolver.refs))
}

struct Resolver<'a> {
    raw: &'a BTreeMap<String, Value>,
    env: &'a BTreeMap<String, String>,
    done: BTreeMap<String, Value>,
    refs: BTreeMap<String, Vec<String>>,
    stack: Vec<String>,
}

impl Resolver<'_> {
    fn resolve(&mut self, path: &str) -> Result<Value> {
        if let Some(value) = self.done.get(path) {
            return Ok(value.clone());
        }
        if let Some(position) = self.stack.iter().position(|p| p == path) {
            let mut cycle = self.stack.get(position..).unwrap_or_default().to_vec();
            cycle.push(path.to_string());
            return Err(Error::InterpolationCycle {
                cycle: cycle.join(" -> "),
            });
        }
        let Some(raw) = self.raw.get(path).cloned() else {
            return Err(Error::MissingReference {
                path: self.stack.last().cloned().unwrap_or_default(),
                reference: path.to_string(),
            });
        };
        let Some(text) = raw.as_str() else {
            self.done.insert(path.to_string(), raw.clone());
            return Ok(raw);
        };

        if !text.contains("${") {
            let value = Value::str(text);
            self.done.insert(path.to_string(), value.clone());
            return Ok(value);
        }

        self.stack.push(path.to_string());
        let result = self.expand(text, path);
        self.stack.pop();

        let value = result?;
        self.done.insert(path.to_string(), value.clone());
        Ok(value)
    }

    fn expand(&mut self, text: &str, path: &str) -> Result<Value> {
        let placeholders = find(text).map_err(|snippet| Error::MalformedPlaceholder {
            path: path.to_string(),
            snippet,
        })?;

        // Una cadena que es exactamente un placeholder hereda el tipo del destino.
        if placeholders.len() == 1 {
            if let Some(only) = placeholders.first() {
                if only.start == 0 && only.end == text.len() {
                    let only = only.clone();
                    return self.eval(&only, path);
                }
            }
        }

        let mut out = String::with_capacity(text.len());
        let mut cursor = 0usize;
        for placeholder in &placeholders {
            let literal = text.get(cursor..placeholder.start).unwrap_or_default();
            out.push_str(&unescape(literal));
            let value = self.eval(placeholder, path)?;
            out.push_str(&value.to_string());
            cursor = placeholder.end;
        }
        out.push_str(&unescape(text.get(cursor..).unwrap_or_default()));
        Ok(Value::str(out))
    }

    fn eval(&mut self, placeholder: &Placeholder<'_>, path: &str) -> Result<Value> {
        match placeholder.target {
            Target::Env(name) => match self.env.get(name) {
                Some(value) => Ok(Value::str(value.as_str())),
                None => self.fallback(placeholder, path, &format!("env:{name}")),
            },
            Target::Key(key) => {
                if !self.raw.contains_key(key) {
                    return self.fallback(placeholder, path, key);
                }
                let value = self.resolve(key)?;
                self.refs
                    .entry(path.to_string())
                    .or_default()
                    .push(key.to_string());
                Ok(value)
            }
        }
    }

    fn fallback(
        &mut self,
        placeholder: &Placeholder<'_>,
        path: &str,
        reference: &str,
    ) -> Result<Value> {
        match placeholder.default {
            Some(default) => self.expand(default, path),
            None => Err(Error::MissingReference {
                path: path.to_string(),
                reference: reference.to_string(),
            }),
        }
    }
}

fn unescape(literal: &str) -> String {
    literal.replace("\\${", "${")
}

fn starts_with_at(bytes: &[u8], at: usize, pattern: &[u8]) -> bool {
    bytes.len() >= at + pattern.len() && &bytes[at..at + pattern.len()] == pattern
}

fn find(input: &str) -> std::result::Result<Vec<Placeholder<'_>>, String> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\\' && starts_with_at(bytes, i + 1, b"${") {
            i += 3;
            continue;
        }
        if starts_with_at(bytes, i, b"${") {
            let close = matching_brace(input, i)?;
            let inner = input.get(i + 2..close).unwrap_or_default();
            out.push(parse_inner(inner, i, close + 1)?);
            i = close + 1;
        } else {
            i += 1;
        }
    }

    Ok(out)
}

fn matching_brace(input: &str, open: usize) -> std::result::Result<usize, String> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut i = open;

    while i < bytes.len() {
        if starts_with_at(bytes, i, b"${") {
            depth += 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok(i);
            }
        }
        i += 1;
    }

    Err(input.get(open..).unwrap_or_default().to_string())
}

fn parse_inner(
    inner: &str,
    start: usize,
    end: usize,
) -> std::result::Result<Placeholder<'_>, String> {
    let (target, default) = split_default(inner);
    if target.is_empty() {
        return Err(inner.to_string());
    }
    let target = match target.strip_prefix("env:") {
        Some(name) if !name.is_empty() => Target::Env(name),
        Some(_) => return Err(inner.to_string()),
        None => Target::Key(target),
    };
    Ok(Placeholder {
        start,
        end,
        target,
        default,
    })
}

fn split_default(inner: &str) -> (&str, Option<&str>) {
    let bytes = inner.as_bytes();
    let mut depth = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if starts_with_at(bytes, i, b"${") {
            depth += 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'}' {
            depth = depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if depth == 0 && bytes[i] == b':' && bytes.get(i + 1) == Some(&b'-') {
            let head = inner.get(..i).unwrap_or_default();
            let tail = inner.get(i + 2..).unwrap_or_default();
            return (head, Some(tail));
        }
        i += 1;
    }

    (inner, None)
}

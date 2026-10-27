//! `${...}` reference interpolation.
//!
//! Semantics (specification §8): references and pure resolvers only, no expressions, no
//! `eval`. Supports `${key}`, `${key:-default}`, `${env:VAR}` and nesting inside default
//! values. Resolution is memoised and cycles are detected with a full trace.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Error, Result};
use crate::resolve::Resolvers;
use crate::source::{Segment, render_path};
use crate::value::{Node, Value};

/// Origin of a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target<'a> {
    Key(&'a str),
    Resolver { name: &'a str, args: Vec<&'a str> },
}

/// Placeholder located inside a string.
#[derive(Debug, Clone)]
struct Placeholder<'a> {
    start: usize,
    end: usize,
    target: Target<'a>,
    default: Option<&'a str>,
}

/// Resolved values and, per key, the direct references used.
pub(crate) type ResolvedMaps = (BTreeMap<String, Value>, BTreeMap<String, Vec<String>>);

/// Resolves every reference in the map of raw values.
///
/// Returns the resolved values and, per key, the direct references it used (for
/// `explain`).
pub(crate) fn resolve_all(
    raw: &BTreeMap<String, Value>,
    env: &BTreeMap<String, String>,
    resolvers: &Resolvers,
) -> Result<ResolvedMaps> {
    let mut resolver = Resolver {
        raw,
        env,
        resolvers,
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
    resolvers: &'a Resolvers,
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

        // A string that is exactly one placeholder inherits the type of its target.
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
        match &placeholder.target {
            Target::Resolver { name, args } => {
                let Some(resolver) = self.resolvers.get(name) else {
                    return Err(Error::UnknownResolver {
                        path: path.to_string(),
                        name: name.to_string(),
                    });
                };
                match resolver.resolve(args.as_slice(), self.env) {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        self.fallback(placeholder, path, &format!("{name}:{}", args.join(",")))
                    }
                }
            }
            Target::Key(key) => {
                if !self.raw.contains_key(*key) {
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
    let target = match target.split_once(':') {
        Some((name, args)) if !name.is_empty() && !name.contains('.') => Target::Resolver {
            name,
            args: args.split(',').map(str::trim).collect(),
        },
        _ => Target::Key(target),
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

/// Resolves every `${...}` in the tree, in place.
///
/// Leaves are collected into a flat map, resolved once (with memoisation and cycle
/// detection) and written back. Structural replacement already happened in the merge, so
/// interpolation cannot resurrect or drop list items.
pub(crate) fn resolve_tree(
    root: &mut Node,
    env: &BTreeMap<String, String>,
    resolvers: &Resolvers,
) -> Result<()> {
    let mut raw = BTreeMap::new();
    collect_leaves(root, &mut Vec::new(), &mut raw);

    let (values, refs) = resolve_all(&raw, env, resolvers)?;
    apply_leaves(root, &mut Vec::new(), &values, &refs);
    propagate_sensitivity(root);
    Ok(())
}

/// A value built from a sensitive reference is sensitive too.
fn propagate_sensitivity(root: &mut Node) {
    let mut sensitive = BTreeSet::new();
    collect_sensitive(root, &mut Vec::new(), &mut sensitive);
    if sensitive.is_empty() {
        return;
    }

    let mut refs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    collect_refs(root, &mut Vec::new(), &mut refs);

    loop {
        let mut changed = false;
        for (path, targets) in &refs {
            if sensitive.contains(path) {
                continue;
            }
            if targets.iter().any(|target| sensitive.contains(target)) {
                sensitive.insert(path.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    apply_sensitivity(root, &mut Vec::new(), &sensitive);
}

fn collect_sensitive(node: &Node, segments: &mut Vec<Segment>, out: &mut BTreeSet<String>) {
    if node.sensitive {
        out.insert(render_path(segments));
    }
    match &node.value {
        Value::Map(entries) => {
            for (key, child) in entries {
                segments.push(Segment::Key(key.clone()));
                collect_sensitive(child, segments, out);
                segments.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter().enumerate() {
                segments.push(Segment::Index(index));
                collect_sensitive(child, segments, out);
                segments.pop();
            }
        }
        _ => {}
    }
}

fn collect_refs(node: &Node, segments: &mut Vec<Segment>, out: &mut BTreeMap<String, Vec<String>>) {
    match &node.value {
        Value::Map(entries) => {
            for (key, child) in entries {
                segments.push(Segment::Key(key.clone()));
                collect_refs(child, segments, out);
                segments.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter().enumerate() {
                segments.push(Segment::Index(index));
                collect_refs(child, segments, out);
                segments.pop();
            }
        }
        _ => {
            if !node.refs.is_empty() {
                out.insert(render_path(segments), node.refs.clone());
            }
        }
    }
}

fn apply_sensitivity(node: &mut Node, segments: &mut Vec<Segment>, sensitive: &BTreeSet<String>) {
    match &mut node.value {
        Value::Map(entries) => {
            for (key, child) in entries.iter_mut() {
                segments.push(Segment::Key(key.clone()));
                apply_sensitivity(child, segments, sensitive);
                segments.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter_mut().enumerate() {
                segments.push(Segment::Index(index));
                apply_sensitivity(child, segments, sensitive);
                segments.pop();
            }
        }
        _ => {
            if sensitive.contains(&render_path(segments)) {
                node.sensitive = true;
            }
        }
    }
}

fn collect_leaves(node: &Node, segments: &mut Vec<Segment>, out: &mut BTreeMap<String, Value>) {
    match &node.value {
        Value::Map(entries) => {
            for (key, child) in entries {
                segments.push(Segment::Key(key.clone()));
                collect_leaves(child, segments, out);
                segments.pop();
            }
        }
        Value::Seq(items) => {
            for (index, child) in items.iter().enumerate() {
                segments.push(Segment::Index(index));
                collect_leaves(child, segments, out);
                segments.pop();
            }
        }
        value => {
            out.insert(render_path(segments), value.clone());
        }
    }
}

fn apply_leaves(
    node: &mut Node,
    segments: &mut Vec<Segment>,
    values: &BTreeMap<String, Value>,
    refs: &BTreeMap<String, Vec<String>>,
) {
    match &mut node.value {
        Value::Map(entries) => {
            for (key, child) in entries.iter_mut() {
                segments.push(Segment::Key(key.clone()));
                apply_leaves(child, segments, values, refs);
                segments.pop();
            }
            return;
        }
        Value::Seq(items) => {
            for (index, child) in items.iter_mut().enumerate() {
                segments.push(Segment::Index(index));
                apply_leaves(child, segments, values, refs);
                segments.pop();
            }
            return;
        }
        _ => {}
    }

    let key = render_path(segments);
    if let Some(value) = values.get(&key) {
        node.value = value.clone();
    }
    node.refs = refs.get(&key).cloned().unwrap_or_default();
}

//! Python binding (PyO3) for `keydex`.
//!
//! Thin layer: it implements no semantics, it only translates. The engine builds an
//! immutable snapshot and `Settings` only reads it; `refresh()` builds another one and
//! swaps it atomically.
//!
//! `extension-module` is always on, so this crate **cannot** be linked by
//! `cargo test --workspace` (it needs libpython). It is checked with `cargo check -p
//! keydex-py` and linked by maturin.
//!
//! The module declares `gil_used = false`: there is no mutable global state (the snapshot
//! lives in an `ArcSwap`) and every operation is safe without the GIL.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;

use keydex_core::{
    Builder, Error, Explanation, Format, Provenance, Schema, Snapshot, Tier, ValidationError,
    Value, coerce, unknown_keys,
};
use pyo3::IntoPyObjectExt;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

create_exception!(keydex, KeydexError, PyException);

/// Everything needed to rebuild the snapshot in `refresh()`.
#[derive(Debug, Clone)]
struct Spec {
    files: Vec<String>,
    profiles: Vec<String>,
    env_prefix: String,
    env: BTreeMap<String, String>,
    sensitive: Vec<String>,
    /// Environment variables whose bound key must be marked sensitive (`*_FILE`).
    sensitive_env: Vec<String>,
    /// Schema IR when the caller asked for a typed load; `None` in dictionary mode.
    schema: Option<String>,
    /// Unknown-key policy, kept so `refresh()` applies the same one.
    on_unknown: String,
    /// `true` when the environment was captured by us and `refresh()` must re-read it.
    refresh_env: bool,
    /// `true` when `*_FILE` variables must be resolved by reading the files.
    secret_files: bool,
    /// Directory that resolved `*_FILE` paths must stay inside.
    secret_root: Option<String>,
}

/// Resolved configuration, immutable and queryable by key.
#[pyclass(name = "Settings", module = "keydex", frozen)]
struct PySettings {
    spec: Spec,
    /// Lock-free reads: `load_full` is an atomic bump, only `refresh` swaps.
    inner: ArcSwap<Snapshot>,
}

impl PySettings {
    fn snapshot(&self) -> Arc<Snapshot> {
        self.inner.load_full()
    }

    /// Rebuilds the spec for a refresh, re-reading the process environment and the secret
    /// files when they were captured by us.
    fn fresh_spec(&self) -> PyResult<Spec> {
        let mut spec = self.spec.clone();
        // The initial load only reads the process environment when there is a prefix;
        // capturing it here without one would make refresh() change the meaning of every
        // key named like a common variable (HOME, PATH, USER).
        if !spec.refresh_env || spec.env_prefix.is_empty() {
            return Ok(spec);
        }

        // Non-UTF-8 entries are skipped rather than panicking: `std::env::vars` panics on
        // them, and a configuration library has no business taking the process down.
        let mut env: BTreeMap<String, String> = std::env::vars_os()
            .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
            .collect();

        if spec.secret_files {
            let candidates: Vec<String> = env
                .keys()
                .filter(|key| {
                    let upper = key.to_ascii_uppercase();
                    upper.ends_with("_FILE")
                        && (spec.env_prefix.is_empty()
                            || upper.starts_with(&spec.env_prefix.to_ascii_uppercase()))
                })
                .cloned()
                .collect();

            // No `*_FILE` variable, no reason to touch the filesystem: a `secret_root` that
            // no longer exists must not turn a refresh into a failure.
            if !candidates.is_empty() {
                let root = spec.secret_root.as_deref().ok_or_else(|| {
                    KeydexError::new_err("secret_root is required when secret_files is enabled")
                })?;
                let root = std::fs::canonicalize(expand_tilde(root))
                    .map_err(|err| KeydexError::new_err(format!("secret_root {root}: {err}")))?;

                for key in candidates {
                    let reference = env[&key].clone();
                    let resolved =
                        std::fs::canonicalize(expand_tilde(&reference)).map_err(|err| {
                            KeydexError::new_err(format!("{key}: cannot read {reference}: {err}"))
                        })?;
                    if !resolved.starts_with(&root) {
                        return Err(KeydexError::new_err(format!(
                            "{key}: {} is outside secret_root ({})",
                            resolved.display(),
                            root.display()
                        )));
                    }
                    let contents = std::fs::read_to_string(&resolved).map_err(|err| {
                        KeydexError::new_err(format!(
                            "{key}: cannot read {}: {err}",
                            resolved.display()
                        ))
                    })?;
                    let stem = key[..key.len() - 5].to_string();
                    if !spec.sensitive_env.contains(&stem) {
                        spec.sensitive_env.push(stem.clone());
                    }
                    env.insert(stem, contents.trim().to_string());
                    env.remove(&key);
                }
            }
        }

        spec.env = env;
        Ok(spec)
    }
}

#[pymethods]
impl PySettings {
    /// Returns the value of a key, or `default` when it does not exist.
    #[pyo3(signature = (key, default = None))]
    fn get(&self, py: Python<'_>, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self.snapshot().get(key) {
            Some(value) => to_py(py, value),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Same as `get`, but raises `KeyError` when the key does not exist.
    ///
    /// Sensitive keys return `None`, so `dict(settings)` cannot leak them. Use `get()` when
    /// you need the raw value.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        let snapshot = self.snapshot();
        match snapshot.get(key) {
            Some(value) => {
                if snapshot.is_sensitive(key) {
                    Ok(py.None())
                } else {
                    to_py(py, value)
                }
            }
            None => Err(PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        Ok(self.snapshot().get(key).is_some())
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.snapshot().len())
    }

    /// Keys in lexicographic order.
    fn keys(&self) -> PyResult<Vec<String>> {
        Ok(self.snapshot().keys().map(ToString::to_string).collect())
    }

    /// `(key, value)` pairs in lexicographic order.
    ///
    /// Sensitive keys are returned as `None`: use `get()` when you need the value.
    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let snapshot = self.snapshot();
        snapshot
            .iter()
            .map(|(key, value)| {
                let item = if snapshot.is_sensitive(key) {
                    py.None()
                } else {
                    to_py(py, value)?
                };
                Ok((key.to_string(), item))
            })
            .collect()
    }

    /// Provenance of a key, or `None` when it does not exist.
    ///
    /// Sensitive keys report `value: None` and `redacted: true` unless `reveal` is set.
    #[pyo3(signature = (key, reveal = false))]
    fn explain(&self, py: Python<'_>, key: &str, reveal: bool) -> PyResult<Py<PyAny>> {
        let snapshot = self.snapshot();
        match snapshot.explain(key) {
            Some(explanation) => {
                explanation_to_py(py, &explanation, snapshot.is_sensitive(key) && !reveal)
            }
            None => Ok(py.None()),
        }
    }

    /// Re-reads the sources and replaces the snapshot.
    ///
    /// If anything fails the current snapshot is kept: configuration is never left half
    /// applied.
    fn refresh(&self) -> PyResult<()> {
        let spec = self.fresh_spec()?;
        let (snapshot, schema) = build_snapshot(&spec)?;
        if let Some(schema) = schema {
            if let Err(errors) = coerce(&snapshot, &schema) {
                let summary = errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(KeydexError::new_err(summary));
            }
            if spec.on_unknown == "error" {
                let unknown = unknown_keys(&snapshot, &schema);
                if !unknown.is_empty() {
                    let summary = unknown
                        .iter()
                        .map(|item| format!("{}: unknown key", item.path))
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(KeydexError::new_err(summary));
                }
            }
        }
        self.inner.store(Arc::new(snapshot));
        Ok(())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("<keydex.Settings keys={}>", self.snapshot().len()))
    }
}

#[pyfunction]
#[pyo3(signature = (files, env_prefix = "", env = None, profiles = Vec::new(), sensitive = Vec::new(), sensitive_env = Vec::new(), refresh_env = true, secret_files = false, secret_root = None))]
/// Loads one or several files and applies environment variables and profiles.
///
/// `sensitive` lists the keys that must be hidden from `explain`, `items` and `dump`.
fn load(
    files: Vec<String>,
    env_prefix: &str,
    env: Option<HashMap<String, String>>,
    profiles: Vec<String>,
    sensitive: Vec<String>,
    sensitive_env: Vec<String>,
    refresh_env: bool,
    secret_files: bool,
    secret_root: Option<String>,
) -> PyResult<PySettings> {
    let spec = Spec {
        files,
        profiles,
        env_prefix: env_prefix.to_string(),
        env: match env {
            Some(map) => map.into_iter().collect(),
            None => std::env::vars().collect(),
        },
        sensitive,
        sensitive_env,
        schema: None,
        on_unknown: String::from("warn"),
        refresh_env,
        secret_files,
        secret_root,
    };
    let (snapshot, _) = build_snapshot(&spec)?;
    Ok(PySettings {
        spec,
        inner: ArcSwap::from_pointee(snapshot),
    })
}

#[pyfunction]
#[pyo3(signature = (ir, files, env_prefix = "", env = None, profiles = Vec::new(), on_unknown = "warn", sensitive = Vec::new(), sensitive_env = Vec::new(), refresh_env = true, secret_files = false, secret_root = None))]
/// Loads with a schema in a single build: the typed tree and the settings share one
/// snapshot, so `explain` describes exactly what the model was built from.
///
/// Returns `(settings, errors, warnings, tree)`.
fn load_typed(
    py: Python<'_>,
    ir: &str,
    files: Vec<String>,
    env_prefix: &str,
    env: Option<HashMap<String, String>>,
    profiles: Vec<String>,
    on_unknown: &str,
    sensitive: Vec<String>,
    sensitive_env: Vec<String>,
    refresh_env: bool,
    secret_files: bool,
    secret_root: Option<String>,
) -> PyResult<(PySettings, Vec<Py<PyAny>>, Vec<Py<PyAny>>, Py<PyAny>)> {
    let spec = Spec {
        files,
        profiles,
        env_prefix: env_prefix.to_string(),
        env: match env {
            Some(map) => map.into_iter().collect(),
            None => std::env::vars().collect(),
        },
        sensitive,
        sensitive_env,
        schema: Some(ir.to_string()),
        on_unknown: on_unknown.to_string(),
        refresh_env,
        secret_files,
        secret_root,
    };

    let (snapshot, schema) = build_snapshot(&spec)?;
    let Some(schema) = schema else {
        return Err(KeydexError::new_err("the typed path requires a schema"));
    };

    let coerced = coerce(&snapshot, &schema);
    let unknown = unknown_keys(&snapshot, &schema);

    let mut errors: Vec<ValidationError> = Vec::new();
    let mut warnings: Vec<(String, String)> = Vec::new();

    let payload = match coerced {
        Ok(tree) => {
            for item in unknown {
                let message = match &item.suggestion {
                    Some(suggestion) => {
                        format!("unknown key, did you mean {suggestion:?}?")
                    }
                    None => String::from("unknown key"),
                };
                match on_unknown {
                    "ignore" => {}
                    "error" => errors.push(ValidationError {
                        path: item.path,
                        message,
                        span: item.span,
                    }),
                    _ => warnings.push((item.path, message)),
                }
            }
            if errors.is_empty() {
                json_to_py(py, &tree)?
            } else {
                py.None()
            }
        }
        Err(list) => {
            errors.extend(list);
            py.None()
        }
    };

    let settings = PySettings {
        spec,
        inner: ArcSwap::from_pointee(snapshot),
    };

    Ok((
        settings,
        errors_to_py(py, &errors)?,
        pairs_to_py(py, &warnings)?,
        payload,
    ))
}

fn build_snapshot(spec: &Spec) -> PyResult<(Snapshot, Option<Schema>)> {
    let mut builder = Builder::new().with_env(spec.env.clone());
    for name in &spec.sensitive_env {
        builder.mark_sensitive_env(name);
    }

    let schema = match &spec.schema {
        Some(ir) => {
            let schema = Schema::from_json(ir).map_err(to_py_err)?;
            builder
                .schema_env_source(&schema, &spec.env_prefix, Tier::Environment)
                .map_err(to_py_err)?;
            Some(schema)
        }
        None => {
            builder.env_source(&spec.env_prefix, Tier::Environment);
            None
        }
    };

    add_documents(&mut builder, spec)?;
    for key in &spec.sensitive {
        builder.mark_sensitive(key);
    }

    let snapshot = builder.build().map_err(to_py_err)?;
    Ok((snapshot, schema))
}

/// Adds the base files and, after them, those of every active profile.
///
/// Convention: `app.yaml` + profile `prod` -> `app-prod.yaml`, in the same directory. If
/// the profile file does not exist it is ignored; the last profile in the list wins.
fn add_documents(builder: &mut Builder, spec: &Spec) -> PyResult<()> {
    for path in &spec.files {
        let (contents, format) = read_document(path)?;
        builder
            .document(path, &contents, format, Tier::BaseFile)
            .map_err(to_py_err)?;
    }

    for profile in &spec.profiles {
        for base in &spec.files {
            let Some(path) = profile_path(base, profile) else {
                continue;
            };
            let (contents, format) = read_document(&path)?;
            builder
                .document(&path, &contents, format, Tier::ProfileFile)
                .map_err(to_py_err)?;
        }
    }

    Ok(())
}

fn read_document(path: &str) -> PyResult<(String, Format)> {
    let contents = std::fs::read_to_string(path)
        .map_err(|err| KeydexError::new_err(format!("{path}: {err}")))?;
    let format = Format::from_path(path)
        .ok_or_else(|| KeydexError::new_err(format!("unknown format for {path}")))?;
    Ok((contents, format))
}

/// Expands a leading `~` the way the Python side does, so a refresh resolves the same
/// paths the first load did.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(path)
}

/// `dir/app.yaml` + `prod` -> `dir/app-prod.yaml`, when it exists.
fn profile_path(base: &str, profile: &str) -> Option<String> {
    let path = Path::new(base);
    let stem = path.file_stem()?.to_str()?;
    let extension = path.extension()?.to_str()?;
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let candidate: PathBuf = parent.join(format!("{stem}-{profile}.{extension}"));
    candidate
        .exists()
        .then(|| candidate.to_str().map(ToString::to_string))
        .flatten()
}

fn to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py_any(py)?,
        Value::Int(i) => i.into_py_any(py)?,
        Value::Float(f) => f.into_py_any(py)?,
        Value::Str(s) => s.into_py_any(py)?,
        Value::Seq(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(to_py(py, &item.value)?)?;
            }
            list.into_any().unbind()
        }
        Value::Map(entries) => {
            let dict = PyDict::new(py);
            for (key, child) in entries {
                dict.set_item(key, to_py(py, &child.value)?)?;
            }
            dict.into_any().unbind()
        }
        // `Value` is `non_exhaustive`: a future variant is exposed as `None` until this
        // layer translates it.
        _ => py.None(),
    })
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_py_any(py)?,
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(integer) => integer.into_py_any(py)?,
            None => number.as_f64().unwrap_or_default().into_py_any(py)?,
        },
        serde_json::Value::String(text) => text.into_py_any(py)?,
        serde_json::Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, json_to_py(py, item)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

fn to_py_err(error: Error) -> PyErr {
    KeydexError::new_err(error.to_string())
}

fn errors_to_py(py: Python<'_>, errors: &[ValidationError]) -> PyResult<Vec<Py<PyAny>>> {
    errors
        .iter()
        .map(|error| {
            let dict = PyDict::new(py);
            dict.set_item("path", &error.path)?;
            dict.set_item("message", &error.message)?;
            match error.span.as_ref() {
                Some(span) => {
                    dict.set_item("file", &span.file)?;
                    dict.set_item("line", span.line)?;
                    dict.set_item("col", span.col)?;
                }
                None => {
                    dict.set_item("file", py.None())?;
                    dict.set_item("line", py.None())?;
                    dict.set_item("col", py.None())?;
                }
            }
            Ok(dict.into_any().unbind())
        })
        .collect()
}

fn pairs_to_py(py: Python<'_>, pairs: &[(String, String)]) -> PyResult<Vec<Py<PyAny>>> {
    pairs
        .iter()
        .map(|(path, message)| {
            let dict = PyDict::new(py);
            dict.set_item("path", path)?;
            dict.set_item("message", message)?;
            Ok(dict.into_any().unbind())
        })
        .collect()
}

fn provenance_to_py(py: Python<'_>, provenance: &Provenance) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("source", &provenance.source)?;
    dict.set_item("tier", provenance.tier.rank())?;
    match provenance.span.as_ref() {
        Some(span) => {
            dict.set_item("file", &span.file)?;
            dict.set_item("line", span.line)?;
            dict.set_item("col", span.col)?;
        }
        None => {
            dict.set_item("file", py.None())?;
            dict.set_item("line", py.None())?;
            dict.set_item("col", py.None())?;
        }
    }
    Ok(dict.into_any().unbind())
}

fn explanation_to_py(
    py: Python<'_>,
    explanation: &Explanation,
    redact: bool,
) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("key", &explanation.key)?;
    if redact {
        dict.set_item("value", py.None())?;
        dict.set_item("redacted", true)?;
    } else {
        dict.set_item("value", to_py(py, &explanation.value)?)?;
        dict.set_item("redacted", false)?;
    }
    dict.set_item("winner", provenance_to_py(py, &explanation.winner)?)?;
    let shadowed: Vec<Py<PyAny>> = explanation
        .shadowed
        .iter()
        .map(|provenance| provenance_to_py(py, provenance))
        .collect::<PyResult<_>>()?;
    dict.set_item("shadowed", shadowed)?;
    dict.set_item("refs", explanation.refs.clone())?;
    Ok(dict.into_any().unbind())
}

#[pymodule(gil_used = false)]
fn _keydex(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySettings>()?;
    module.add_function(wrap_pyfunction!(load, module)?)?;
    module.add_function(wrap_pyfunction!(load_typed, module)?)?;
    module.add("KeydexError", module.py().get_type::<KeydexError>())?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

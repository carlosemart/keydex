//! Binding de Python (PyO3) para `keydex`.
//!
//! Capa fina: no implementa semántica, solo traduce. El motor construye un snapshot
//! inmutable y `Settings` solo lo consulta; `refresh()` construye otro y lo intercambia
//! de forma atómica.
//!
//! `extension-module` está activo siempre, así que este crate **no** se puede enlazar en
//! `cargo test --workspace` (requiere libpython). Se compila con `cargo check -p
//! keydex-py` y se enlaza desde maturin.
//!
//! TODO(F4): declarar `gil_used = false` cuando se publique el wheel `cp314t`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use keydex_core::{
    Builder, Error, Explanation, Format, Provenance, Schema, Snapshot, Tier, Value, coerce,
    unknown_keys,
};
use pyo3::IntoPyObjectExt;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

create_exception!(keydex, KeydexError, PyException);

/// Todo lo necesario para reconstruir el snapshot en `refresh()`.
#[derive(Debug, Clone)]
struct Spec {
    files: Vec<String>,
    profiles: Vec<String>,
    env_prefix: String,
    env: BTreeMap<String, String>,
}

/// Configuración resuelta, inmutable y consultable por clave.
#[pyclass(name = "Settings", module = "keydex", frozen)]
struct PySettings {
    spec: Spec,
    inner: Arc<RwLock<Arc<Snapshot>>>,
}

impl PySettings {
    fn snapshot(&self) -> PyResult<Arc<Snapshot>> {
        self.inner
            .read()
            .map(|guard| Arc::clone(&guard))
            .map_err(|_| KeydexError::new_err("estado interno envenenado"))
    }
}

#[pymethods]
impl PySettings {
    /// Devuelve el valor de una clave, o `default` si no existe.
    #[pyo3(signature = (key, default = None))]
    fn get(&self, py: Python<'_>, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self.snapshot()?.get(key) {
            Some(value) => to_py(py, value),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Igual que `get`, pero lanza `KeyError` si la clave no existe.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.snapshot()?.get(key) {
            Some(value) => to_py(py, value),
            None => Err(PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> PyResult<bool> {
        Ok(self.snapshot()?.get(key).is_some())
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.snapshot()?.len())
    }

    /// Claves en orden lexicográfico.
    fn keys(&self) -> PyResult<Vec<String>> {
        Ok(self.snapshot()?.keys().map(ToString::to_string).collect())
    }

    /// Pares (clave, valor) en orden lexicográfico.
    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let snapshot = self.snapshot()?;
        snapshot
            .iter()
            .map(|(key, value)| Ok((key.to_string(), to_py(py, value)?)))
            .collect()
    }

    /// Procedencia de una clave, o `None` si no existe.
    fn explain(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.snapshot()?.explain(key) {
            Some(explanation) => explanation_to_py(py, &explanation),
            None => Ok(py.None()),
        }
    }

    /// Relee las fuentes y sustituye el snapshot.
    ///
    /// Si algo falla, el snapshot vigente se mantiene: nunca queda configuración a medias.
    fn refresh(&self) -> PyResult<()> {
        let snapshot = build_snapshot(&self.spec)?;
        let mut guard = self
            .inner
            .write()
            .map_err(|_| KeydexError::new_err("estado interno envenenado"))?;
        *guard = Arc::new(snapshot);
        Ok(())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("<keydex.Settings keys={}>", self.snapshot()?.len()))
    }
}

#[pyfunction]
#[pyo3(signature = (files, env_prefix = "", env = None, profiles = Vec::new()))]
/// Carga uno o varios ficheros y les aplica variables de entorno y perfiles.
fn load(
    files: Vec<String>,
    env_prefix: &str,
    env: Option<HashMap<String, String>>,
    profiles: Vec<String>,
) -> PyResult<PySettings> {
    let spec = Spec {
        files,
        profiles,
        env_prefix: env_prefix.to_string(),
        env: match env {
            Some(map) => map.into_iter().collect(),
            None => std::env::vars().collect(),
        },
    };
    let snapshot = build_snapshot(&spec)?;
    Ok(PySettings {
        spec,
        inner: Arc::new(RwLock::new(Arc::new(snapshot))),
    })
}

#[pyfunction]
#[pyo3(signature = (ir, files, env_prefix = "", env = None, profiles = Vec::new(), on_unknown = "warn"))]
/// Carga con esquema: binding relajado, coerción y validación.
///
/// Devuelve `(errores, avisos, arbol)`. La capa de Python decide qué excepción lanzar;
/// el motor no conoce las clases de excepción de Python.
fn load_typed(
    py: Python<'_>,
    ir: &str,
    files: Vec<String>,
    env_prefix: &str,
    env: Option<HashMap<String, String>>,
    profiles: Vec<String>,
    on_unknown: &str,
) -> PyResult<(Vec<Py<PyAny>>, Vec<Py<PyAny>>, Py<PyAny>)> {
    let schema = Schema::from_json(ir).map_err(to_py_err)?;
    let spec = Spec {
        files,
        profiles,
        env_prefix: env_prefix.to_string(),
        env: match env {
            Some(map) => map.into_iter().collect(),
            None => std::env::vars().collect(),
        },
    };

    let mut builder = Builder::new().with_env(spec.env.clone());
    builder
        .schema_env_source(&schema, &spec.env_prefix, Tier::Environment)
        .map_err(to_py_err)?;
    add_documents(&mut builder, &spec)?;

    let snapshot = builder.build().map_err(to_py_err)?;

    let coerceado = coerce(&snapshot, &schema);
    let desconocidas = unknown_keys(&snapshot, &schema);

    let mut errors: Vec<(String, String)> = Vec::new();
    let mut warnings: Vec<(String, String)> = Vec::new();

    let payload = match coerceado {
        Ok(tree) => {
            for item in desconocidas {
                let message = match &item.suggestion {
                    Some(suggestion) => {
                        format!("clave desconocida, ¿quisiste decir {suggestion:?}?")
                    }
                    None => String::from("clave desconocida"),
                };
                match on_unknown {
                    "ignore" => {}
                    "error" => errors.push((item.path, message)),
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
            errors.extend(list.into_iter().map(|error| (error.path, error.message)));
            py.None()
        }
    };

    Ok((
        pairs_to_py(py, &errors)?,
        pairs_to_py(py, &warnings)?,
        payload,
    ))
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

fn build_snapshot(spec: &Spec) -> PyResult<Snapshot> {
    let mut builder = Builder::new().with_env(spec.env.clone());
    builder.env_source(&spec.env_prefix, Tier::Environment);
    add_documents(&mut builder, spec)?;
    builder.build().map_err(to_py_err)
}

/// Añade los ficheros base y, después, los de cada perfil activo.
///
/// Convención: `app.yaml` + perfil `prod` → `app-prod.yaml`, en el mismo directorio. Si
/// el fichero de perfil no existe se ignora; el último perfil de la lista gana.
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
        .ok_or_else(|| KeydexError::new_err(format!("formato desconocido para {path}")))?;
    Ok((contents, format))
}

/// `dir/app.yaml` + `prod` → `dir/app-prod.yaml`, si existe.
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
        // `Value` es `non_exhaustive`: un valor futuro se expone como `None` hasta que
        // esta capa lo traduzca.
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

fn explanation_to_py(py: Python<'_>, explanation: &Explanation) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("key", &explanation.key)?;
    dict.set_item("value", to_py(py, &explanation.value)?)?;
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

#[pymodule]
fn _keydex(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySettings>()?;
    module.add_function(wrap_pyfunction!(load, module)?)?;
    module.add_function(wrap_pyfunction!(load_typed, module)?)?;
    module.add("KeydexError", module.py().get_type::<KeydexError>())?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

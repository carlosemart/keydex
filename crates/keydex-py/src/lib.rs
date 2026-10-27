//! Binding de Python (PyO3) para `keydex`.
//!
//! Es una capa fina: no implementa semántica, solo traduce. La regla dura 5 manda que
//! ningún acceso a atributo cruce la frontera FFI más de lo imprescindible: el motor
//! construye un snapshot inmutable y `Settings` solo lo consulta.
//!
//! `extension-module` está activo siempre, así que este crate **no** se puede enlazar en
//! `cargo test --workspace` (requiere libpython). Se compila con `cargo check -p
//! keydex-py` y se enlaza desde maturin.
//!
//! TODO(F4): declarar `gil_used = false` cuando se publique el wheel `cp314t`.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use keydex_core::{Builder, Error, Explanation, Format, Provenance, Snapshot, Tier, Value};
use pyo3::IntoPyObjectExt;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

create_exception!(keydex, KeydexError, PyException);

/// Configuración resuelta, inmutable y consultable por clave.
#[pyclass(name = "Settings", module = "keydex", frozen)]
struct PySettings {
    inner: Arc<Snapshot>,
}

#[pymethods]
impl PySettings {
    /// Devuelve el valor de una clave, o `default` si no existe.
    #[pyo3(signature = (key, default = None))]
    fn get(&self, py: Python<'_>, key: &str, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        match self.inner.get(key) {
            Some(value) => to_py(py, value),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Igual que `get`, pero lanza `KeyError` si la clave no existe.
    fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.inner.get(key) {
            Some(value) => to_py(py, value),
            None => Err(PyKeyError::new_err(key.to_string())),
        }
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Claves en orden lexicográfico.
    fn keys(&self) -> Vec<String> {
        self.inner.keys().map(ToString::to_string).collect()
    }

    /// Pares (clave, valor) en orden lexicográfico.
    fn items(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        self.inner
            .iter()
            .map(|(key, value)| Ok((key.to_string(), to_py(py, value)?)))
            .collect()
    }

    /// Procedencia de una clave, o `None` si no existe.
    fn explain(&self, py: Python<'_>, key: &str) -> PyResult<Py<PyAny>> {
        match self.inner.explain(key) {
            Some(explanation) => explanation_to_py(py, &explanation),
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        format!("<keydex.Settings keys={}>", self.inner.len())
    }
}

#[pyfunction]
#[pyo3(signature = (files, env_prefix = "", env = None))]
/// Carga uno o varios ficheros de configuración y les aplica las variables de entorno.
///
/// `env` permite inyectar un entorno propio (tests); si se omite, se usa el del proceso.
fn load(
    files: Vec<String>,
    env_prefix: &str,
    env: Option<HashMap<String, String>>,
) -> PyResult<PySettings> {
    let environment: BTreeMap<String, String> = match env {
        Some(map) => map.into_iter().collect(),
        None => std::env::vars().collect(),
    };

    let mut builder = Builder::new().with_env(environment);
    builder.env_source(env_prefix, Tier::Environment);

    for path in files {
        let contents = std::fs::read_to_string(&path)
            .map_err(|err| KeydexError::new_err(format!("{path}: {err}")))?;
        let format = Format::from_path(&path)
            .ok_or_else(|| KeydexError::new_err(format!("formato desconocido para {path}")))?;
        builder
            .document(&path, &contents, format, Tier::BaseFile)
            .map_err(to_py_err)?;
    }

    let snapshot = builder.build().map_err(to_py_err)?;
    Ok(PySettings {
        inner: Arc::new(snapshot),
    })
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
    module.add("KeydexError", module.py().get_type::<KeydexError>())?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

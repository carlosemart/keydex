# 0003 — Wheels: abi3-py311 + cp314t

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

Free-threading (PEP 703) no existe en 3.12: llega en 3.13 como build experimental
(`python3.13t`) y es oficial en 3.14 (PEP 779). Los wheels `abi3` no cargan en
intérpretes free-threaded, y un `abi3t` (PEP 803) no está disponible.

## Decisión

- Mínimo Python 3.11 (da `tomllib` en stdlib, sin depender de `tomli`).
- Un wheel `abi3-py311` cubre 3.11–3.14 con GIL.
- Wheels separados `cp314t` (y opcionalmente `cp313t`) para free-threading.
- `#[pymodule(gil_used = false)]` y estado global bajo `RwLock`.

## Consecuencias

- Dos perfiles de build con maturin; matriz de CI más ancha, sin código duplicado.
- El snapshot inmutable ya es la arquitectura correcta para free-threading.

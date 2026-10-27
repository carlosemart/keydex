# 0003 — Wheels: abi3-py311 + cp314t

- Status: accepted
- Date: 2026-10-27

## Context

Free-threading (PEP 703) does not exist in 3.12: it lands in 3.13 as an experimental build
(`python3.13t`) and is official in 3.14 (PEP 779). `abi3` wheels do not load in
free-threaded interpreters, and `abi3t` (PEP 803) is not available yet.

## Decision

- Python 3.11 minimum (gives `tomllib` in the standard library, no `tomli` dependency).
- One `abi3-py311` wheel covers 3.11–3.15 with the GIL.
- Separate `cp314t` wheels (and optionally `cp313t`) for free-threading.
- `#[pymodule(gil_used = false)]` and global state behind `RwLock`.

## Consequences

- Two build profiles with maturin; a wider CI matrix with no duplicated code.
- The immutable snapshot is already the right architecture for free-threading.

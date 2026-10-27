# keydex-core

Configuration engine of [keydex](https://github.com/carlosemart/keydex): sources,
precedence, merge, interpolation, coercion and provenance.

Pure Rust, with no Python dependency. Meant to be embedded in Rust services and to serve as
the core of the Python library.

## Invariants

- Does not depend on `pyo3`.
- Does not read the environment, the clock or the filesystem: everything is injected.
- Structured errors (enums), never `String`.
- No mutable global state.

## Status

Pre-release. The public API is not stable yet.

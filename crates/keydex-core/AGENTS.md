# keydex-core

This crate is published on crates.io: minimal public API, `#[non_exhaustive]` on public
enums, semver 0.x, docs.rs with `all-features`.

- Zero `pyo3` and zero direct reads of the environment, clock or filesystem: everything is
  injected.
- Errors are enums (`thiserror`), never strings.
- No `unwrap`/`expect`/`panic` outside `#[cfg(test)]`.
- Any `std::env`, `std::time::SystemTime` or `std::fs` method is blocked by `clippy.toml`;
  if you need one, the solution is an injectable abstraction.
- Every public item needs documentation: `missing_docs` is set to `deny`.

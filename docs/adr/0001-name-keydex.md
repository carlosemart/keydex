# 0001 — Name: keydex

- Status: accepted
- Date: 2026-10-27

## Context

The English words in this domain (config, env, settings, merge, layer, canon, provenance,
whence, clew…) are taken on PyPI and crates.io. `keydex` is free on both registries.

## Decision

`keydex` as the project name, the Python package name (`import keydex`) and the repo name.
Crates: `keydex-core` (published) and `keydex-py` (not published).

## Consequences

- GitHub has unrelated repositories with the same name (one of them, a TUI password
  manager with ~99 stars). The identifiers that matter for installing are PyPI and
  crates.io, and there the name is ours.
- `pip install keydex` and `cargo add keydex-core` are unambiguous.

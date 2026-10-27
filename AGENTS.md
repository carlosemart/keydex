# keydex — project instructions

Specification: `docs/spec.md`. Decisions: `docs/adr/`. Read them before touching
`crates/`. If code and spec disagree, the spec wins: ask before changing it.

## What keydex is

Configuration access: typed, with provenance, and fast. You ask for a key, you get the
right value, from the right origin, with the right type, and you can ask why.

## What it is NOT (do not add it even if it looks useful)

DI container · expression engine · application framework · multi-language FFI · hot reload
by default · validation in Python.

## Hard rules

1. `keydex-core` does NOT depend on `pyo3`.      → `./scripts/gate.sh`
2. `keydex-core` does NOT read environment, clock or FS. → `./scripts/gate.sh` + clippy
3. Errors are `thiserror` enums. Never `String`. → clippy `panic`/`unwrap_used`
4. No mutable global state; caches behind `RwLock`.
5. No attribute access crosses FFI; only `build` and `refresh`.
6. Every key exposed in `explain` keeps its span.

## Prohibitions

- No `serde_yaml` or `serde_yml` (archived, no spans): use `saphyr`/`marked_yaml`.
- No `unwrap`/`expect`/`panic` outside `#[cfg(test)]`.
- No new dependency without an ADR and `cargo deny`.
- No change to `keydex.schema/v1` without a bump, a shared fixture and a cross test.
- No `eval`, no network, no arbitrary file reads from a resolver.

## Map

```
crates/keydex-core  pure engine: sources, precedence, merge, interp, coercion
crates/keydex-py    PyO3 binding (F1), publish = false
python/keydex       public API, types and integrations (F1)
docs/spec.md        current specification
docs/adr/           irreversible decisions
```

## Commands

```
./scripts/check.sh     # fmt + clippy -D warnings
cargo test --workspace --exclude keydex-py
./scripts/gate.sh      # architecture invariants (hard rules 1 and 2)
```

## Git

- **Never `git push` to `main`.** It is protected: branch plus PR, always.
- One branch per change: `feat/…`, `fix/…`, `docs/…`, `chore/…`.
- Before opening the PR: `./scripts/check.sh`, `cargo test --workspace --exclude
  keydex-py` and `python -m pytest python/tests examples/homelab-demo/tests`.
- Do not rewrite shared history (`--force`) and do not publish (tag, PyPI, crates.io)
  without explicit user approval.

## Done when

- [ ] there is a test that fails without the change
- [ ] `./scripts/check.sh` and `./scripts/gate.sh` are green, and
      `cargo test --workspace --exclude keydex-py` passes
- [ ] no performance regression once a benchmark exists
- [ ] `docs/spec.md` updated if behaviour changed
- [ ] entry in `CHANGELOG.md`

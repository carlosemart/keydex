# Contributing

Read [AGENTS.md](https://github.com/carlosemart/keydex/blob/main/AGENTS.md) first: the hard rules there are not suggestions, and most of
them are enforced by a command. This page is the human version of the workflow.

## Setup

```bash
mise run python:install   # venv + editable install
mise run ci               # everything a pull request must pass
```

Without `mise`, the scripts work directly: `./scripts/check.sh`, `cargo test --workspace
--exclude keydex-py`, `./scripts/gate.sh`, `.venv/bin/python -m pytest python/tests
examples/homelab-demo/tests`.

`cargo install cargo-deny --locked` makes the gate match CI.

## Layout

| Path | What lives there |
|---|---|
| `crates/keydex-core` | the engine. No `pyo3`, no filesystem, no environment |
| `crates/keydex-py` | PyO3 binding: file IO, environment capture, conversions |
| `python/keydex` | public API, dataclass introspection, `Secret`, CLI |
| `docs/` | specification, architecture, guides, ADRs, status |
| `examples/homelab-demo` | runnable end-to-end example |

## Adding a resolver

1. Implement `keydex_core::Resolver` in the core (built-in) or in the caller (custom).
2. Register it: `builder.resolver(Arc::new(MyResolver))`.
3. Test both the happy path and an unknown/invalid argument, in
   `crates/keydex-core/tests/extension.rs`.

## Adding a source

1. Implement `keydex_core::Source`, returning a `Node` tree and a `SourceId` with the
   `Tier` and order you want.
2. Pass it with `builder.source(Box::new(...))`.
3. Leave provenance alone: the builder stamps it.

## Adding a field type

This is the most invasive change; do it in this order:

1. `crates/keydex-core/src/schema.rs`: add the variant to `FieldType`, then handle it in
   `convert`, `walk` (structural validation), `collect_paths` (environment binding) and
   `collect_leaf_paths` (unknown keys).
2. `python/keydex/_schema.py`: emit the new IR kind and construct the Python object in
   `_convert`.
3. Tests: a valid value, an invalid one (expect `ValidationError` with a path) and, if it is
   a rich type, a round-trip.
4. `docs/status.md` and `docs/spec.md` §7.

## Changing behaviour

The specification is the contract. A behaviour change edits `docs/spec.md` **in the same
pull request**, and adds or updates the test that proves it.

## Running the docs

```bash
mise run docs         # site/ and target/doc/
mise run docs:serve   # live preview on :8000
```

`mkdocs build --strict` fails on a broken link, so it runs in CI.

## Pull requests

- One branch per change: `feat/…`, `fix/…`, `docs/…`, `chore/…`.
- Six checks must be green: `lint`, `test`, `gate`, `python`, `python-freethreaded`, `docs`.
- `main` is protected; direct pushes are rejected.
- Commit messages: imperative subject, body explains *why*, not *what*.

## Where to start

`docs/status.md` lists what is missing. The safest first contributions are a missing type, a
troubleshooting entry, or a test that reproduces a bug you hit.

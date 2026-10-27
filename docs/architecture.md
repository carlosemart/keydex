# Architecture

How keydex is built. The [specification](spec.md) says what it must do; this page says how
the pieces fit and why they are where they are.

## Overview

Two Rust crates and one Python package:

```
┌─────────────────────────────────────────────────────────────────────┐
│ python/keydex                                                       │
│   Environment · load() · load_settings() · Secret                    │
│   dataclass introspection → IR · result → dataclass                  │
│   environment capture · *_FILE resolution · warnings                 │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ PyO3 (one call per build / refresh)
┌───────────────────────────────▼─────────────────────────────────────┐
│ crates/keydex-py   (extension "keydex._keydex")                      │
│   Settings (ArcSwap<Snapshot>) · file IO · profile file discovery    │
│   value → Python · error → exception · refresh                       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ Rust API
┌───────────────────────────────▼─────────────────────────────────────┐
│ crates/keydex-core                                                   │
│   Builder ──► Source trait ──► Node tree                             │
│            ──► merge ──► interpolation (Resolver trait) ──► Snapshot │
│   schema: IR, binding, coercion, unknown keys                        │
│   parsers: YAML / TOML / JSON with spans                             │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
        saphyr · toml_edit · jsonc-parser · serde_json
```

Dependencies only point downwards. `keydex-core` has no `pyo3`, no `std::env`, no
`std::fs` and no clock: everything enters as a value. That is what makes it publishable and
embeddable on its own.

## Modules

| Module | Responsibility | Must not know |
|---|---|---|
| `value` | `Value` (scalars, `Seq`, `Map`), `Node`, `Span`, `Provenance` | sources, schema |
| `source` | `Tier`, `SourceId`, `Path`/`Segment`, the `Source` trait | how data is obtained |
| `merge` | structural merge and path insertion | schema, Python |
| `parse` | YAML/TOML/JSON → `Node` with spans; `LineIndex` | precedence |
| `env` | captured environment → path/value pairs (dictionary mode) | schema |
| `resolve` | `Resolver` trait, registry, built-in `env` | files |
| `interp` | placeholder grammar, topological resolution, cycles, sensitivity propagation | I/O |
| `loader` | `Builder`: sources, ordering, merge, interpolation, snapshot | Python |
| `schema` | IR parsing and validation, relaxed binding, coercion, unknown keys | Python types |
| `snapshot` | immutable tree, leaf index, `explain` | building |
| `error` | structured errors | formatting |

Python side:

| File | Responsibility |
|---|---|
| `python/keydex/__init__.py` | public API, `Environment`, environment capture, `*_FILE`, warnings |
| `python/keydex/_schema.py` | dataclass → IR, typed tree → dataclass |
| `python/keydex/_secrets.py` | `Secret[T]` presentation |
| `python/keydex/__main__.py` | CLI (`explain`, `dump`) |
| `crates/keydex-py` | the only place that does file I/O and the only FFI boundary |

## Data model

The configuration is a **tree**, not a list of keys:

```rust
Value::Map(BTreeMap<String, Node>)   // merged key by key
Value::Seq(Vec<Node>)                // replaced whole
Value::Str / Int / Float / Bool / Null

Node { value, span, provenance, shadowed, refs, sensitive }
```

`Path` is a list of `Segment::Key | Segment::Index`. A literal key containing a dot is
rendered as `["a.b"]`, so it cannot collide with a nested `a.b`.

Why a tree and not `BTreeMap<String, Value>` of flat paths:

- a list is a value, so a higher-priority source can **replace** it, and `[]` clears it;
- an empty map or list is representable;
- a scalar can replace a whole subtree instead of leaving stale children;
- there is no ambiguous key grammar.

The flat `keys()` view is derived at snapshot build time for iteration and `explain`.

## Lifecycle of a load

Typed, through `Environment`:

1. `Environment.load(Model)` builds the IR from the dataclass (`_schema.build_ir`).
2. `_environment` copies the environment (or the injected one) and, only when
   `secret_files=True`, resolves `*_FILE` under `secret_root`, recording the derived keys
   as sensitive.
3. One call into `load_typed(ir, files, prefix, env, profiles, on_unknown, sensitive)`.
4. In Rust, `Spec` → `build_snapshot`:
   1. parse the IR and validate it;
   2. bind the environment to schema paths (relaxed, exact-first);
   3. read the documents and their profile files, parse into trees;
   4. stamp provenance, mark sensitive paths;
   5. merge by tier, resolve interpolation and sensitivity;
   6. build the `Snapshot`.
5. `coerce` produces the typed JSON tree; `unknown_keys` produces warnings.
6. The tree becomes Python values, `_schema.build` constructs the dataclass, and the same
   `Snapshot` is kept inside `Settings` for `explain`/`refresh`.

Dictionary mode is the same minus steps 1 and 5.

`refresh()` reruns step 4 with the stored `Spec` (including the IR), revalidates, and only
then swaps the pointer. If it fails, the previous snapshot keeps serving.

## Merge

| Low → High | Result |
|---|---|
| map, map | merge key by key, recursively |
| seq, seq | high wins |
| anything, anything else | high wins; the whole low subtree is shadowed |
| missing, x | x |

Sensitivity is OR-ed; interpolation marks a value sensitive if any of its references is.

## Extension points

| Seam | Status |
|---|---|
| `Source` | open: implement and pass `Box<dyn Source>` to `Builder::source` |
| `Resolver` | open: implement and pass `Arc<dyn Resolver>` to `Builder::resolver` |
| `Format` | **closed enum** (`yaml`, `toml`, `json`); adding one means editing the core |

Provenance is stamped by the builder, so a custom source only declares its `SourceId`.

## Cross-cutting concerns

**Errors.** `keydex_core::Error` is a `thiserror` enum. Validation errors are aggregated as
`ValidationError { path, message, span }`; the binding turns them into
`keydex.ValidationError` with `file`, `line` and `col`. Parse failures keep the parser
message. Nothing embeds a raw value in a message unless the value is public.

**Sensitivity.** `Node.sensitive` is set by the caller or by a `*_FILE` key, survives the
merge, and propagates through interpolation. `explain`, `items`, `dump` and the CLI redact
it; `reveal` is the only way out; `get()` is the raw escape hatch. `Secret[T]` is the typed
presentation of the same idea.

**Determinism.** Sources are sorted by `(tier desc, order asc)`; every map is a `BTreeMap`;
`keys()` follows the leaf order. Same input, same output.

**Concurrency.** The snapshot is immutable and `Sync`. `Settings` holds it in an
`ArcSwap`: readers do not take a lock, `refresh()` swaps the pointer atomically. The module
declares `gil_used = false`.

**I/O.** Only `keydex-py` and `python/keydex/__init__.py` touch the filesystem. The core
receives contents.

## Decisions

| ADR | Decision |
|---|---|
| [0001](adr/0001-name-keydex.md) | name |
| [0002](adr/0002-no-expressions.md) | interpolation without expressions |
| [0003](adr/0003-wheels-abi3-and-314t.md) | `abi3` and free-threaded builds |
| [0004](adr/0004-schema-ir-versioning.md) | IR versioned apart from the package |
| [0005](adr/0005-mandatory-spans.md) | spans are mandatory |
| [0006](adr/0006-parsers-with-spans.md) | `saphyr`, `toml_edit`, `jsonc-parser` |
| [0007](adr/0007-schema-ir-over-serde.md) | the IR travels as JSON |
| [0008](adr/0008-arc-swap-lock-free-reads.md) | lock-free reads |

## Known gaps

- Sensitivity of a typed model is derived from its `Secret[T]` fields when loading through
  `Environment`; a bare `load()` returns a dataclass, so there is no `explain` to redact.
- `Format` is not pluggable.
- Profiles, file discovery and `*_FILE` live in the binding, so a pure Rust caller does not
  get them.
- No `watch()`, no remote sources, no `imports:`.

## Reading order

For a newcomer: [Python guide](python.md) → this page → `crates/keydex-core/src/loader.rs`
and `schema.rs` → the [specification](spec.md) for the full contract.

# keydex — Specification

> A **configuration access** library for Python, with a Rust core.
> Typed, explainable and fast: you ask for a key, you get the right value, from the right
> origin, with the right type, and you can ask **why**.

Status: closed specification · Date: 2026-10-27 · Release: 0.1.0

Sections marked **Planned** describe the agreed design but are not implemented yet. The
code and this document must not disagree: if a behaviour changes, this file changes in the
same pull request.

---

## 0. Summary

`keydex` solves "where does this configuration value come from?" with a typed API and a Rust
engine. It takes the best of Spring `Environment` (property sources, profiles, relaxed
binding, explicit precedence) and of OmegaConf (interpolation, composition, a missing-value
sentinel), without paying for a tree of Python objects per node and without `eval`.

**Main differentiator:** every value in the snapshot knows its full provenance (source,
file, line, placeholder chain, who was shadowed). That is what turns the library into an
operations tool rather than just a loader.

---

## 1. Closed decisions

| Decision | Value |
|---|---|
| Name | `keydex` (`keydex-core`, `keydex-py`, `import keydex`) |
| Main API | typed `load(Model)` plus `env.get()` as an escape hatch |
| Python | 3.11+ · `abi3-py311` wheel · `cp314t` wheel for free-threading |
| Scope | Python only; `keydex-core` is published to crates.io |
| Precedence | Ordered bands plus insertion order inside a band |
| Binding | Relaxed, resolved against the schema (see §6) |
| Interpolation | References and resolvers only. **No expressions, no `eval`** |
| Composition | `imports`/`includes` planned; Hydra-style groups as a future plugin |
| Reload | `refresh()` in v1; `watch()` in v2 |
| Split | Rust: parsing, merge, interpolation, coercion, validation, provenance. Python: types, ergonomics, integrations |
| Performance | Zero FFI per access; shared immutable snapshot |
| Non-goals | DI container, expressions, multi-language FFI, wasm, hot reload by default |

---

## 2. Invariant principles

Rules that are not negotiable; they are cheap now and very expensive later.

1. **`keydex-core` does not know Python.** `cargo tree -p keydex-core` must not contain
   `pyo3`. Enforced in CI as a blocking gate.
2. **The core does not read the environment, the clock or the filesystem.** Everything is
   injected. That is what makes it testable, embeddable and publishable as a useful crate.
3. **Structured errors, never `String`.** `thiserror` enums; each layer formats them (an
   exception in Python, JSON in the CLI).
4. **No mutable global state.** No configuration `static`, no cache without a lock. One
   effort covers both threads and free-threading.
5. **One FFI crossing per high-level operation.** `build()` and `refresh()` cross the
   boundary; accesses do not.

---

## 3. Conceptual model

```
          ┌──────────────── PropertySource (band + order) ─────────────────┐
          │  YAML · TOML · JSON · env vars · CLI · secrets                 │
          └───────────────────────────────┬───────────────────────────────┘
                                          │ bytes + spans
                                          ▼
                                    Parse (Rust)
                                          ▼
                                  Value tree with spans
                                          ▼
                    Deterministic merge (§9) + profile activation (§10)
                                          ▼
                    Interpolation graph → topological order (§8)
                                          ▼
                    Coercion + validation against the Schema IR (§7, §11)
                                          ▼
                    ┌──── Immutable snapshot + provenance (§13) ────┐
                    │  flat keys · interning · immutable             │
                    └───────────────────┬───────────────────────────┘
                                        ▼
                          Typed decode (once)  |  Mapping (dict mode)
```

- **`PropertySource`**: pluggable unit with a `tier` (band) and an `order` (position inside
  the band). Precedence is not a hardcoded rigid list.
- **`Snapshot`**: flat key tree, immutable, shareable between threads, with an atomic swap
  in `refresh()`.
- **`Schema IR`**: serialisable description of the model that Python generates and Rust
  consumes. It is the compatibility contract between both worlds.

---

## 4. Precedence

Bands from highest to lowest priority. Inside a band, ascending `order`; the last one
applied wins.

| Band | Origin | Status | Notes |
|---|---|---|---|
| 0 | Test / programmatic overrides | Planned | `env.override()`, pytest fixture |
| 1 | Mapped CLI arguments | Planned | `--db-host=x` → `db.host` |
| 2 | OS environment variables | Implemented | `APP_` prefix, `*_FILE` for secrets |
| 3 | Files of the active profile | Implemented | the last profile wins |
| 4 | Base files | Implemented | the last location wins |
| 5 | Schema defaults | Not implemented | `???` = required and absent (a sentinel, not `None`) |
| 6 | Code defaults | Via Python | declared in the dataclass |

Only three bands exist in the engine today. The Python layer adds an environment source
(band 2) and base/profile documents (bands 4 and 3); defaults come from the dataclass, not
from the engine.

Additional rules:

- A source may declare a reserved `tier` (0–6) or be inserted **before/after another
  specific source** (`insert_before="env"`). This avoids the classic "my source does not
  fit". *Planned.*
- An imported document (`config.import`) is treated as **immediately below** the importing
  document, not as an independent source. *Planned (§9.3).*
- The order is **deterministic** and does not depend on `os.environ` order or on the
  filesystem. Same input → same `dump()` byte for byte.

---

## 5. Sources

### 5.1 Local formats

**Implemented:** YAML (through `saphyr`), TOML (through `toml_edit`) and JSON (through
`jsonc-parser`, which also accepts JSONC).

**Planned:** INI, `.properties`, `.env` and YAML multi-document.

Technical requirement: parsers must preserve **spans** (line/column) because `explain`
exposes them. That drives the dependency choice (§17.3).

### 5.2 File discovery

- Explicit `files` list. **Implemented.**
- Profile files by convention: `app.yaml` + `prod` → `app-prod.yaml`, in the same directory.
  **Implemented.**
- `search_paths`, `file:`/`optional:` URI prefixes and environment-based discovery.
  *Planned.*
- `imports:` inside a document, resolved in depth with cycle detection. *Planned (§9.3).*

### 5.3 Environment

- Configurable prefix (`env_prefix="APP_"`), stripped and lowercased. **Implemented.**
- Syntax: see §6.
- `*_FILE`: if `APP_DB_PASSWORD_FILE` exists **and `secret_files=True`**, the value is
  read from the file, must live under `secret_root`, and the derived key is marked
  sensitive. Off by default: without opting in, `*_FILE` variables are inert and no file is
  read. **Implemented.**

### 5.4 Other sources

CLI arguments, `random.*` (int, uuid, port) per boot, and remote sources through plugins
(Vault, KMS, S3, Consul, etcd, keyring, 1Password, Bitwarden, SOPS). *Planned, v1.1+.*

---

## 6. Relaxed binding

With a schema, binding **does not guess**: the schema disambiguates.

### 6.1 Key normalisation

`dbHost`, `db-host`, `db_host`, `DB_HOST` are the same key. The canonical form is
`snake_case` with `.` between levels and `[i]` for indices.

### 6.2 Environment variables → keys

| Environment variable (`APP_` stripped) | Result |
|---|---|
| `DB_HOST` | `db.host` (resolved against the schema) |
| `DB__HOST` | `db.host` (explicit level) |
| `SERVERS__0__HOST` | `servers[0].host` *(environment binding of lists: planned)* |
| `LABELS__TEAM_A` | `labels.team_a` *(dynamic maps: planned)* |

Implemented algorithm:

1. Strip the prefix (case-insensitive) and lowercase the remainder.
2. Replace `__` with `.` to obtain the **dotted** form.
3. Look up the dotted form in the map of schema paths. If it matches, that is the binding.
4. Otherwise look up the **relaxed** form (dots replaced by `_`).
5. If two schema paths tie in the relaxed form → `AmbiguousBinding` error listing both
   candidates.
6. An exact dotted match always beats a relaxed one.

**Dictionary mode (no schema):** `__` is mandatory as a level separator and `_` is literal.
Deterministic and heuristic-free. That is why typed mode is the main path.

### 6.3 Collisions

If two model fields normalise to the same name, the IR is rejected at `load()` time, not at
runtime. *Planned: explicit validation of the IR.*

---

## 7. Types and coercion

### 7.1 Supported types

**Implemented:** `str`, `int`, `float`, `bool`, `Optional[T]`, `list[T]`, nested
dataclasses, `Enum`, `Path`, `Decimal`, `UUID`, `Secret[T]`.

**Planned:** `datetime`/`date`/`time`, `IPv4Address`/`IPv6Address`, `duration`, `size`,
`rate`, `URL`, discriminated `Union`, `Literal`, `tuple`, `set`, `dict[str, T]`, `attrs`,
`pydantic` (through JSON Schema), `msgspec.Struct`, `TypedDict`, `NamedTuple`.

### 7.2 Coercion rules

| Type | Accepts (`relaxed` mode) | Caveat |
|---|---|---|
| `bool` | `true/false`, `yes/no`, `on/off`, `"1"/"0"` as **text** | A YAML/TOML/JSON integer `1` is rejected, not coerced |
| `int` | `42`, `"42"` | |
| `float` | `3.14`, `"3.14"`, integers | |
| `Enum` | exact member value | Case-insensitive matching is **not** implemented; an invalid value raises `ValueError` from Python, not `ValidationError` |
| `Path` | `str`; `~` is **not** expanded | |
| `Decimal` | exact `str` | Sourced from a YAML/JSON number it goes through the text form and can lose precision (`1.10` → `1.1`) |
| `datetime` | ISO-8601 | *Planned* |
| `duration` | `250ms`, `5s`, `3m`, `2h`, `1d`, `PT5M` | *Planned* |
| `size` | `10MB`, `1GiB` (binary vs decimal explicit) | *Planned* |
| `rate` | `100/s`, `5MB/s` | *Planned* |

Modes: `strict` (lossless conversions only), `relaxed` (default, table above), `lenient`
(best effort). **Never** `"false"` → `True`, never a lossy float → int, never silent string
truncation. *Only `relaxed` is implemented today.*

### 7.3 `MISSING` sentinel

`???` means "required and absent". It is not `None`. Only `MISSING` triggers a required
error; `Optional[T]` accepts an explicit `None` from configuration. **Planned:** today a
missing scalar is a required error, and `Optional` without a value resolves to `None`.

---

## 8. Interpolation

### 8.1 Syntax

| Form | Meaning | Status |
|---|---|---|
| `${db.host}` | Absolute reference | Implemented |
| `${db.host:-localhost}` | With a default value | Implemented |
| `${env:HOME}` | `env` resolver | Implemented |
| `${env:NOPE:-fallback}` | Resolver with default | Implemented |
| `${a:-${b}}` | Nested defaults | Implemented |
| `\${...}` | Literal escape | Implemented |
| `${.sibling}` | Relative reference (sibling) | Planned |
| `${..parent.key}` | Upward relative reference | Planned |

A string that is exactly one placeholder inherits the type of the referenced value;
`"${...}"` inside a larger string is rendered as text.

### 8.2 Semantics

- Resolved **once, in topological order**, memoised in the snapshot → zero cost per access.
- Cycles → error with the **full trace** of the cycle, not a generic message.
- Reference to a missing key without a default → `MissingReference` error.
- Values that are never read are still resolved, so dead references fail at load time.

### 8.3 Resolvers

`env` is implemented. **Planned:** `base64`, `int`, `float`, `bool`, `list`, `json`,
`uuid`, and Python-registered resolvers.

- Arguments separated by `,`, escapable with `\,`. *Planned.*
- `file:` would read arbitrary paths: disabled by default and never enabled implicitly.
- A resolver must be **pure**. No network and no global state from the core.
- There is no expression language: no arithmetic, conditionals or method access. If it is
  ever wanted, it ships as a plugin and never in the core.

---

## 9. Composition and merge

### 9.1 Default rules

- Maps: deep merge, key by key.
- Lists: **replacement** (like Spring). A shorter list drops the extra items, and `[]`
  clears the lower one.
- Scalars: the last one wins.
- A scalar replaces a whole subtree: `db: disabled` drops `db.host` from lower sources.
- A literal key containing a dot or a bracket cannot collide with a nested path: paths are
  typed segments, and the flat form quotes such keys as `["a.b"]`.

All of the above is implemented and covered by tests, including empty containers.

### 9.2 Intent markers *(planned)*

```yaml
servers:
  _merge_: append          # replace | append | prepend | keyed:name
  _items_: [...]
labels:
  _delete_: [obsolete]
```

`keyed:<field>` merges lists of structs by identity, reporting duplicates.

### 9.3 Includes *(planned)*

`imports:` / `includes:` resolved before the merge, with a configurable maximum depth and
cycle detection. An import inherits the importer's active profile unless it declares
otherwise.

> **Status:** not implemented, and it cannot live in the core: the calling layer must
> resolve the files, because the core does not touch the filesystem. Pending for F3.1.

---

## 10. Profiles

**Implemented:** activation by file convention plus a dedicated band.

- `app.yaml` is the base (`BaseFile` band); `app-<profile>.yaml` is the overlay
  (`ProfileFile` band, above the base).
- A profile file that does not exist is ignored; an optional profile must not break
  startup.
- With several profiles, the last one in the list wins.
- The environment always wins over any profile (band 2 vs band 3).

**Planned:** activation through configuration (`profiles.active`), environment
(`APP_PROFILES_ACTIVE`) and CLI; `profiles.include`; `profiles.group`; per-document
conditional activation (`activate.on_profile`, `on_property`); a `test` profile shipped as a
pytest fixture.

---

## 11. Schema IR (compatibility contract)

Versioned JSON, generated by Python and consumed by Rust. It is the public surface kept
stable across releases.

```json
{
  "ir": "keydex.schema/v1",
  "root": {
    "type": "struct",
    "required": true,
    "fields": {
      "db": {
        "type": "struct",
        "required": true,
        "fields": {
          "host": { "type": "str", "required": true },
          "port": { "type": "int", "required": false }
        }
      },
      "note": { "type": "optional", "required": false, "inner": { "type": "str" } },
      "tags": { "type": "list", "required": false, "items": { "type": "str" } },
      "mode": { "type": "str", "required": false }
    }
  }
}
```

Types in v1: `str`, `int`, `float`, `bool`, `any`, `struct`, `optional`, `list`. The engine
is language neutral: it knows nothing about host language types. Rich conversions
(`Enum`, `Path`, `Decimal`, `UUID`, `Secret`) are applied by the calling layer after
coercion, based on its own type information.

Evolution rules:

- Inside `v1`, only **additive** changes (new optional fields).
- Any break → `keydex.schema/v2` with simultaneous support.
- The IR is also the basis for exported JSON Schema, `keydex schema`, and (future)
  validation from other languages.

Fields with a default and a missing value are omitted from the coerced tree so the
dataclass default applies: a single source of truth for defaults.

---

## 12. Validation and errors

Implemented:

- Type validation and `required` detection from the IR.
- **Aggregated errors**, not fail-fast. Shape per error:

```python
{"path": "db.port", "message": "expected an integer, got \"many\""}
```

- Unknown keys: `on_unknown = "warn" | "error" | "ignore"` (default `warn`), with an
  edit-distance suggestion (`db.prot` → *unknown key, did you mean "db.port"?*). The
  suggestion also works when the level is nested wrongly (`app.name` → `name`).
- Exception hierarchy: `KeydexError` → `ValidationError`; `UnknownKeyWarning` is a
  `UserWarning`, not an exception.

**Planned:** ranges, regex, lengths, cardinality, custom validators, and a finer
exception hierarchy (`SchemaError`, `ParseError`, `BindingError`, `InterpolationError`,
`SecretError`, `SourceError`).

---

## 13. Provenance

`explain(key)` returns an object, not a formatted string (formatting belongs to each
frontend):

```python
cfg.explain("db.host")
# {
#   "key": "db.host",
#   "value": "db.internal",
#   "redacted": False,
#   "winner":   {"source": "env", "tier": 2, "file": None, "line": None, "col": None},
#   "shadowed": [{"source": "file:app.yaml", "tier": 4,
#                 "file": "app.yaml", "line": 4, "col": 8}],
#   "refs": ["db.suffix"],
# }
```

Rules:

- `shadowed` includes **every** origin that defined the key and lost, oldest first.
- **Redaction depends on the caller.** A key declared `sensitive` (via `sensitive=[...]`
  or derived from `*_FILE`) is returned with `value: None` and `redacted: True`;
  `reveal=True` shows it. Keys not declared sensitive are returned in the clear: in
  dictionary mode there is no schema to know which values are secret. `get()` and `[]`
  always return the raw value.
- `dump` is deterministic and uses `items()`, which applies the same redaction.
- `explain` exists on `Settings` (dictionary mode). A typed model is a plain dataclass and
  has no provenance; load with `load_settings` when you need it.
- CLI equivalent: `python -m keydex explain db.host` (add `--reveal` to see sensitive
  values).

---

## 14. Secrets

- `Secret[T]`: `repr`, `str`, `format`, logs and traces → `Secret('***')`. The real value
  only through `.reveal()`.
- `*_FILE` indirection for Docker/K8s secrets.
- Typed models accept `Secret[str]`; the IR marks it with `converter: "secret"`.
- Non-leak guarantees verified by tests: `repr`, the demo output, and the dataclass.

**Planned:** `zeroize` for buffers holding secrets in Rust, redaction in `dump`/`explain`,
and remote secret sources (keyring, Vault, 1Password, Bitwarden, SOPS).

---

## 15. Lifecycle and reload

- `load()` builds an immutable snapshot.
- `refresh()` (dictionary mode): re-reads the sources, validates and **only then** swaps
  atomically. If it fails, the current snapshot keeps serving and the error is reported.
  Configuration is never left half applied. **Implemented.**
- A typed model is an immutable copy: to reload, call `load()` again.
- Thread safety: shared immutable snapshot; the `Settings` handle holds it behind an
  `RwLock` that is only taken on `refresh()`.
- `watch()` (v2): `notify` plus debounce, `on_change(keys)` callbacks.
- Per-request overrides through `contextvar`. *Planned.*

---

## 16. Python API

```python
from dataclasses import dataclass, field
from keydex import Secret

@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432
    password: Secret[str] | None = None

@dataclass(frozen=True)
class App:
    db: Db
    debug: bool = False

cfg = keydex.load(App, files=["app.yaml"], env_prefix="APP_", profiles=["prod"])
cfg.db.host                 # native access, no FFI
```

```python
def load(
    model: type[T],
    *,
    files: Sequence[str] = (),
    env_prefix: str = "",
    env: Mapping[str, str] | None = None,
    profiles: Sequence[str] = (),
    on_unknown: Literal["warn", "error", "ignore"] = "warn",
) -> T: ...

def load_settings(
    files: Sequence[str] = (),
    env_prefix: str = "",
    env: Mapping[str, str] | None = None,
    profiles: Sequence[str] = (),
) -> Settings: ...

class Settings(Mapping[str, Any]):
    def get(self, key: str, default: Any = None) -> Any: ...
    def explain(self, key: str) -> dict | None: ...
    def refresh(self) -> None: ...
```

- **No global singleton.** Call `load` where the configuration is needed and pass it as a
  dependency.
- `env` lets a test inject its own environment; when omitted, `os.environ` is used.
- CLI: `python -m keydex explain|dump`.

**Planned integrations:** pytest fixture, `logging`/`structlog` reconfiguration, FastAPI
(`Depends` + lifespan), Click/Typer, Django settings shim.

---

## 17. Rust architecture

### 17.1 Crates

```
keydex/
├── Cargo.toml                # workspace
├── crates/
│   ├── keydex-core           # pure Rust, publishable, zero pyo3
│   └── keydex-py             # PyO3: adapters and error mapping
├── python/keydex/            # pure Python layer
├── python/tests/             # Python integration tests
└── python/bench/             # load benchmark
```

### 17.2 `keydex-core` modules

| Module | Responsibility |
|---|---|
| `source` | `Tier`, `SourceId`, raw entries |
| `parse` | Span-preserving parsers (YAML, TOML, JSON) plus `LineIndex` |
| `env` | Dictionary-mode environment source |
| `loader` | `Builder`: precedence, merge, provenance assembly |
| `interp` | Interpolation graph, cycles, resolvers |
| `schema` | Schema IR (serde), coercion, validation, relaxed binding, unknown keys |
| `snapshot` | Immutable snapshot and `Explanation` |
| `error` | `thiserror` enums |

### 17.3 Key dependencies

- **YAML with spans:** `saphyr` 0.3 (bundles `marked_yaml`). Confirmed in
  [spike 0001](spikes/0001-parsers-spans.md).
- **TOML with spans:** `toml_edit` 0.25, using `Document` (not `DocumentMut`, which drops
  spans).
- **JSON with positions:** `jsonc-parser` 0.34. `jiter` was discarded: faster, but it
  exposes no positions.
- **One `LineIndex` per file** to convert offsets into line/column; without it the
  conversion is O(n) per key and the parse becomes quadratic.
- `serde` + `serde_json` for the IR (behind the `schema` feature).
- `thiserror`. **Planned:** `compact_str`/`arcstr`, `rayon`, `zeroize`/`secrecy`.
- `pyo3` **only** in `keydex-py`.

### 17.4 Core feature flags

`yaml`, `toml`, `json`, `schema` (all on by default). **Planned:** `ini`, `dotenv`,
`zeroize`, `watch` (v2), `http` (v2). A Rust service that only reads TOML should not pull
half an ecosystem.

### 17.5 Python boundary

- `build`/`load_typed` and `refresh()` are the only expensive crossings.
- Typed decode produces the final Python objects in one pass.
- `gil_used = false` in the PyO3 module; global state behind an `RwLock`.
- `abi3` is a crate feature (`default = ["abi3"]`); the free-threaded build uses
  `--no-default-features`.

---

## 18. Performance

### 18.1 Budgets (CI gates)

| Metric | Target | Measured |
|---|---|---|
| Cold load, 1k YAML keys + env | < 5 ms | 1.26 ms |
| Attribute access | ±10% vs native dataclass (0 FFI) | native |
| `explain()` | < 100 µs | < 10 µs |
| Snapshot memory | < 3× the source text | not measured |
| `import keydex` | < 10 ms | not measured |

**Planned:** mandatory comparison in CI against `omegaconf`, `dynaconf` and
`pydantic-settings`, with `pytest-codspeed` and a regression threshold.

### 18.2 Techniques

Key interning, a path trie, arena allocation, `Cow<str>`, cached topological resolution,
`rayon` for multiple files, GIL released during `build`. **Planned**, except for the
cached resolution which is implemented.

---

## 19. Packaging and CI

### 19.1 Wheels

| Artifact | Covers | Status |
|---|---|---|
| `abi3-py311` | 3.11, 3.12, 3.13, 3.14, 3.15 (with the GIL) | published |
| `cp314t` | 3.14 free-threaded, `gil_used = false` | built and tested in CI, **not published** until reads are lock-free |

`abi3` and `t` are mutually exclusive today (PEP 803 `abi3t` does not exist yet): two build
profiles with maturin.

### 19.2 CI matrix

| Job | Content |
|---|---|
| `lint` | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check -p keydex-py` |
| `test` | `cargo test` on `keydex-core` |
| `gate` | `cargo deny` plus the "no `pyo3` in the core" invariant |
| `python` | builds the wheel with maturin, installs it, runs 44 Python tests |
| `python-freethreaded` | builds `cp314t`, asserts `sys._is_gil_enabled()` is `False`, runs the suite |

**Planned:** fuzzing, benchmarks with regression gates.

### 19.3 Publication

1. `keydex-core` on crates.io (0.x, small API, docs.rs with `all-features`).
2. Wheels on PyPI.
3. Name reservation happens before announcing (manual action, it is a real publication).

Tags drive `.github/workflows/release.yml`:
`v*` → sdist + wheels (x86_64/aarch64, manylinux 2.28) + `cp314t` → PyPI through Trusted
Publishing (OIDC) → `keydex-core` on crates.io through a scoped token. See
[release](release.md).

---

## 20. Testing

Implemented:

- **Precedence tests**: environment over file, order inside a band, shadowing.
- **Binding tests**: the §6.2 table, including the ambiguous case and exact-over-relaxed.
- **Span tests**: YAML, TOML and JSON, with line and column assertions.
- **Interpolation tests**: type inheritance, defaults, escape, cycles, missing references.
- **Typed model tests**: coercion, aggregated errors, unknown keys, rich converters.
- **Secret tests**: no leaks in `repr` or program output.
- **Determinism test**: same input → same key order.

**Planned:** property-based tests (`hypothesis`, `proptest`), fuzzing (`cargo-fuzz`,
`atheris`), concurrency tests from `3.14t`, and a golden dump comparison with shuffled
`os.environ`.

---

## 21. Roadmap

| Phase | Deliverable | Exit criterion |
|---|---|---|
| **F0** ✅ | Workspace, `AGENTS.md` + `docs/spec.md` + gates, parser spans spike, error skeleton | Correct spans and `./scripts/gate.sh` green |
| **F1** ✅ | `keydex-core` (sources, merge, interp, snapshot) + `keydex-py` dict mode | YAML+env load, `${}`, `explain`, benchmark vs OmegaConf |
| **F2** ✅ | Typed schema: dataclasses, coercion, aggregated validation | `load(App)` complete with per-path errors. Generated `.pyi` is not needed: dataclasses already type mypy and the IDE |
| **F3** ✅ | Profiles by file convention, secrets (`Secret`, `*_FILE`), `refresh()`, CLI | Suite green; secrets do not leak in `repr` |
| **F3.1** ✅ | Unknown keys (`on_unknown`) with edit-distance suggestions | Warning raised on a real migration mistake |
| **F4** | Publication: wheels, crates.io, docs | Installable from PyPI on 3.11–3.14t |
| **F5 (v2)** | `watch()`, async remote sources, Hydra-style composition plugin | Only with real demand |

Dogfooding: `examples/homelab-demo` exercises the full surface. The next step is migrating
one real module of a non-Python-shaped project.

---

## 22. Non-goals

- Dependency injection container.
- Spring-style bean lifecycle.
- Expression language in interpolation.
- Multi-language FFI (Go/C#/Java/Kotlin), wasm.
- Hot reload enabled by default.
- Schema mutation at runtime.

---

## 23. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Python↔Rust drift of the Schema IR | Versioned IR + cross tests with shared fixtures |
| Parsers without enough spans | **Resolved in F0**: all three formats give spans ([spike 0001](spikes/0001-parsers-spans.md)) |
| Ambiguous binding in dictionary mode | Strict `__` rule, documented and tested |
| `abi3` vs free-threading diverge | Two build profiles; a dedicated `t` job |
| Scope creep towards a framework | Non-goals in the README and per-phase review |
| Provenance memory overhead | Provenance only for participating keys; interning |

---

## 24. Agent documentation

The working documentation lives in the repository, in four layers:

| Layer | File | Content | Why it exists |
|---|---|---|---|
| Rules | `AGENTS.md` | imperative and verifiable, ≤100 lines | loaded on every session |
| Context | `docs/spec.md` | this specification | the "why" and the contracts |
| Decisions | `docs/adr/NNNN-*.md` | one ADR per irreversible decision | stops re-litigating and losing the reason |
| **Execution** | CI + `./scripts/gate.sh` | what a test can check | **prose is ignored; a red build is not** |

There is also `crates/keydex-core/AGENTS.md`, loaded when working inside that subtree.

### 24.1 Rule → enforcement mechanism

| Rule | Verifiable mechanism |
|---|---|
| core without pyo3 | `./scripts/gate.sh` (pattern `^pyo3( |$)`) plus `--self-test`, which proves the gate fires on a violation |
| core without env/clock/FS | `clippy.toml` blocks **seven specific methods** (`std::env::{var,var_os,vars,vars_os}`, `SystemTime::now`, `std::fs::{read,read_to_string}`), not the whole namespaces; anything else is reviewed |
| structured errors | `deny(panic, unwrap_used, expect_used)` in the core; `ValidationError` still carries a `String` message |
| no global state | `forbid(static_mut_refs)` plus review; the only shared state is the `RwLock` in the binding |
| zero FFI per access | **not enforced yet**: no benchmark gate exists |
| spans in `explain` | golden tests in `crates/keydex-core/tests/snapshot.rs` assert file, line and column |
| banned dependencies | `gate.sh` and `cargo deny` in CI |
| determinism | test compares key order across two builds; a shuffled-environment test is still planned |
| no secret leaks | tests cover `repr` and the redaction of declared sensitive keys in `explain`/`items`; `get()` returns raw values by design |
| no push to `main` | branch protection: PR required, six checks required, force-push blocked |

### 24.2 Rules for `AGENTS.md` itself

- ≤100 lines. If it grows, the agent skims it instead of reading it.
- Everything imperative and checkable: no "preferably" or "consider".
- No duplication of the spec: only rules and pointers to `docs/`.
- Every rule with its verification command next to it.
- Review it when a phase changes: an outdated rule is worse than none.

---

## 25. Annex: name reservation

| Name | PyPI | crates.io |
|---|---|---|
| `keydex` | reserved: 0.0.1 published | not used |
| `keydex-core` | n/a | reserved: 0.0.1 published |
| `keydex-py` | n/a | not published (`publish = false`) |

Reserved on 2026-10-27: `keydex` 0.0.1 on PyPI and `keydex-core` 0.0.1 on crates.io.

---

## 26. Repository and release

### 26.1 A single repository

**`keydex`** (monorepo). Publishing to crates.io and PyPI does not require separate repos:
they are different packages built from the same tag.

The reason is not convenience, it is that **cross tests** only make sense if they live
together:

- shared `keydex.schema/v1` fixtures consumed by Rust and Python;
- the architecture gate (`cargo tree` of `keydex-core`) and the performance gate in the same
  pipeline;
- an IR change breaks both halves at once, and CI sees it in one run.

With separate repos that becomes submodules, duplicated fixtures or desynchronised releases.

### 26.2 Repository layout

```
keydex/
├── .github/workflows/{ci.yml,release.yml}
├── crates/
│   ├── keydex-core/          # publish = true on crates.io
│   └── keydex-py/            # publish = false (distributed through PyPI)
├── python/
│   ├── keydex/               # public API, types, integrations
│   ├── tests/
│   └── bench/
├── docs/
│   ├── spec.md               # this specification
│   ├── python.md             # Python usage guide
│   ├── rust.md               # Rust usage guide
│   ├── benchmarks.md
│   ├── release.md
│   ├── adr/                  # 0001-*.md …
│   └── spikes/               # measured evidence
├── examples/homelab-demo/    # runnable example
├── AGENTS.md
├── CHANGELOG.md
├── SECURITY.md
├── LICENSE-MIT / LICENSE-APACHE
├── README.md
├── Cargo.toml                # workspace
├── Cargo.lock                # committed: we ship binaries and wheels
├── pyproject.toml            # maturin
├── rust-toolchain.toml
├── clippy.toml               # disallowed-methods (hard rule 2)
├── deny.toml                 # bans: serde_yaml
├── mise.toml                 # development tasks
└── .gitignore / .gitattributes
```

### 26.3 When to split into more repositories

| Future repo | When to create it |
|---|---|
| `keydex-hydra` (group composition) | only if the plugin grows and gets its own cadence |
| `keydex-sources-<backend>` | only if a third party maintains the backend |
| `keydex-examples` | never for now: `examples/` is enough |

Rule: **do not split until there is a different release cadence or a different
maintainer**. One repo per concept is CI and versioning debt.

### 26.4 Three different versions (do not confuse them)

| Version | Where it lives | When it changes |
|---|---|---|
| Package (`0.x.y`) | workspace `Cargo.toml` + `pyproject.toml` | every release |
| `keydex.schema/v1` | `ir` field of the JSON | only when the contract breaks |
| Artifact (wheel/`cp314t`) | build tags | ABI changes |

The IR survives several package versions. That is what keeps an old `dump` readable.

### 26.5 Files that cannot be skipped

- **`SECURITY.md`**: the library handles secrets. Disclosure policy from the first commit,
  not after the first incident.
- **Dual license MIT OR Apache-2.0**: Rust ecosystem convention and corporate friendly.
- **`CHANGELOG.md`**: Keep a Changelog format.
- **`docs/adr/`**: the irreversible decisions already taken (name, no expressions,
  `abi3` + `cp314t`, versioned IR, mandatory spans, parser selection, serde transport).

### 26.6 Release flow

1. Conventional commits on a feature branch; `main` is protected.
2. Pull request with the six checks green.
3. Merge → tag `vX.Y.Z`.
4. CI publishes `keydex-core` to crates.io and the sdist plus wheels to PyPI. The order is
   not significant: the wheel is built from source, not from crates.io.

> Name reservation and publication are real actions and are performed by the user, not by
> the agent.

# keydex

Configuration access for Python: **typed, explainable and fast**, with the engine in Rust.

> Status: 0.1.0. Typed models, profiles, secrets, `refresh()` and a CLI. Pending: F4
> publication of wheels and remote sources. PyPI only has a name-reservation placeholder,
> so install from the repository for now.

## Install

There is no public release yet: PyPI only holds the `0.0.1` reservation. `0.1.0` is
published by pushing the `v0.1.0` tag (see [release](docs/release.md)).

In the meantime, from the repository:

```bash
uv venv
uv pip install -e .
```

Published wheels are `abi3` for Python 3.11–3.15 plus a `cp314t` wheel for free-threading.

## Usage

```python
from dataclasses import dataclass, field
import keydex

@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432

@dataclass(frozen=True)
class App:
    db: Db
    debug: bool = False
    tags: list[str] = field(default_factory=list)

# Typed: validates, coerces and returns the model
cfg = keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")
cfg.db.port        # a real int, not "5432"

# Dictionary mode: no model, with provenance
raw = keydex.load_settings(files=["config/app.yaml"], env_prefix="APP_")
raw["db.host"]
raw.explain("db.host")
```

`APP_DB_HOST` matches `db.host` without needing `__`: the schema disambiguates.

Errors are aggregated instead of failing on the first one:

```python
try:
    keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")
except keydex.ValidationError as error:
    error.errors
    # [{"path": "db.port", "message": "expected an integer, got \"many\""}]
```

Keys that no model field uses produce a warning with a suggestion:

```
UnknownKeyWarning: db.prot: unknown key, did you mean "db.port"?
```

Controlled with `on_unknown="warn" | "error" | "ignore"`. It is the safety net against
the quietest failure: a misspelled override that never applies.

The `explain` output is a structure, not text:

```python
{
  "key": "db.host",
  "value": "env.internal",
  "winner": {"source": "env", "tier": 2, "file": None, "line": None, "col": None},
  "shadowed": [
    {"source": "file:config/app.yaml", "tier": 4,
     "file": "config/app.yaml", "line": 2, "col": 8}
  ],
  "refs": [],
}
```

## Profiles

```python
# app.yaml + app-prod.yaml (same directory, suffixed name)
cfg = keydex.load(App, files=["config/app.yaml"], profiles=["prod"])
```

The profile is applied in the `ProfileFile` band, above the base file. A missing profile
file is ignored; with several profiles the last one wins.

## Secrets

```python
from keydex import Secret

@dataclass(frozen=True)
class Db:
    password: Secret[str]

cfg.db.password            # Secret('***')
cfg.db.password.reveal()   # the real value
```

`repr`, `str` and any log of the dataclass show `***`. Docker/K8s indirection is also
supported:

```bash
APP_DB__PASSWORD_FILE=/run/secrets/db_password
```

## Reload

```python
cfg = keydex.load_settings(files=["app.yaml"])
cfg.refresh()      # re-reads; if it fails the current snapshot is kept
```

## CLI

```bash
python -m keydex explain db.host --files config/app.yaml --env-prefix APP_
python -m keydex dump --files config/app.yaml
```

## Why

```python
cfg = keydex.load(AppConfig, profiles=["prod"])
cfg.db.host                 # right value, right type, no FFI per access
cfg.explain("db.host")      # where it comes from, who shadowed it, what it used
```

What it adds over `dynaconf`, `omegaconf` or `pydantic-settings`:

- **First-class provenance**: every value knows which source, file and line defined it.
  It is not an error message, it is a queryable structure.
- **Explicit precedence** by bands, with relative insertion of new sources.
- **Relaxed binding** resolved against the schema, not by heuristics.
- **No `eval`**: reference interpolation and pure resolvers, no expressions.
- **Performance**: parsing, merging and interpolation happen once in Rust; access is
  native Python.

## What it is not

A dependency injection container, an expression engine, an application framework or a
multi-language binding layer. See `AGENTS.md`.

## Documentation

- [Specification](docs/spec.md)
- [Architecture decisions](docs/adr/)
- [Agent instructions](AGENTS.md)
- [Runnable example: homelab service](examples/homelab-demo/)

## Development

```bash
./scripts/check.sh     # fmt + clippy
cargo test --workspace --exclude keydex-py
./scripts/gate.sh      # architecture invariants
```

`scripts/gate.sh` warns and skips `cargo deny` when it is not installed. For CI parity:
`cargo install cargo-deny --locked`.

## License

MIT OR Apache-2.0, at your option.

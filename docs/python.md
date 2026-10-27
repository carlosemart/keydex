# Using keydex from Python

Configuration access that is typed, explainable and fast. The engine is Rust; the API is
plain Python.

## Install

There is no public release yet: PyPI only holds the `0.0.1` name reservation. From the
repository:

```bash
uv venv
uv pip install -e .
```

## Typed models

```python
from dataclasses import dataclass, field
import keydex

@dataclass(frozen=True)
class Db:
    host: str                 # no default: required
    port: int = 5432

@dataclass(frozen=True)
class App:
    db: Db
    debug: bool = False
    tags: list[str] = field(default_factory=list)

cfg = keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")

cfg.db.port        # int 5432, not "5432"
cfg.debug          # bool
```

- Field types come from the dataclass annotations; defaults live in the dataclass and only
  there.
- Supported today: `str`, `int`, `float`, `bool`, `Optional[T]`, `list[T]`, nested
  dataclasses, `Enum`, `Path`, `Decimal`, `UUID`, `Secret[T]`.

## Environment binding

With a model, the schema disambiguates, so you do not need double underscores:

```bash
APP_DB_HOST=db.internal      # -> db.host
APP_DB_PORT=6000             # -> db.port
APP_DB__HOST=db.internal     # also works, and wins over the relaxed form
```

Without a model (dictionary mode) `__` is mandatory: `APP_DB__HOST`.

## Dictionary mode

No model, same engine, plus provenance:

```python
cfg = keydex.load_settings(files=["config/app.yaml"], env_prefix="APP_")

cfg["db.host"]
cfg.get("db.port", 5432)
"db.host" in cfg
len(cfg)
cfg.keys()
dict(cfg.items())
cfg.explain("db.host")
cfg.refresh()          # re-read; on failure the current snapshot is kept
```

`explain` returns a structure:

```python
{
  "key": "db.host",
  "value": "env.internal",
  "winner":   {"source": "env", "tier": 2, "file": None, "line": None, "col": None},
  "shadowed": [{"source": "file:config/app.yaml", "tier": 4,
                "file": "config/app.yaml", "line": 2, "col": 8}],
  "refs": [],
}
```

Tiers: `0` test overrides, `1` CLI, `2` environment, `3` profile file, `4` base file,
`5` schema default, `6` code default. Lower wins.

## Profiles

```python
cfg = keydex.load(App, files=["config/app.yaml"], profiles=["prod"])
```

`app.yaml` is the base; `app-prod.yaml` in the same directory is the overlay. A missing
profile file is ignored. With several profiles, the last one wins. The environment always
wins over any profile.

## Interpolation

```yaml
db:
  host: localhost
  port: 5432
  url: "postgres://${db.host}:${db.port}/app"
name: "${env:APP_NAME:-anonymous}"
```

- Resolved once, after the merge, so `url` follows a `host` overridden by a profile.
- `${key}`, `${key:-default}`, `${env:VAR}`, `${env:VAR:-default}`, nested defaults and
  `\${escaped}` are supported.
- Cycles and missing references (without a default) are errors at load time.

## Secrets

```python
from keydex import Secret

@dataclass(frozen=True)
class Db:
    password: Secret[str]

cfg.db.password            # Secret('***')
cfg.db.password.reveal()   # the real value
```

`repr`, `str` and any log of the dataclass show `***`. Docker/K8s indirection:

```bash
APP_DB__PASSWORD_FILE=/run/secrets/db_password
```

The file is read by the Python layer, not by the engine; if it does not exist, loading
fails instead of starting with an empty password.

## Errors

Validation errors are aggregated, with one entry per problem:

```python
try:
    keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")
except keydex.ValidationError as error:
    for item in error.errors:
        print(item["path"], item["message"])
```

Keys no field uses produce a warning with a suggestion:

```
UnknownKeyWarning: db.prot: unknown key, did you mean "db.port"?
```

Control it with `on_unknown="warn" | "error" | "ignore"`. `"error"` turns the warning into
a `ValidationError` (`path` is the key), which is what you want in CI.

## CLI

```bash
python -m keydex explain db.host --files config/app.yaml --env-prefix APP_
python -m keydex dump --files config/app.yaml
```

Exit codes: `0` success, `1` engine error, `2` unknown key. Useful in scripts and
containers.

## API summary

| Call | Purpose |
|---|---|
| `keydex.load(Model, *, files, env_prefix, env, profiles, on_unknown)` | typed load |
| `keydex.load_settings(files, env_prefix, env, profiles)` | dictionary mode |
| `Settings.get / [] / in / len / keys / items / explain / refresh` | dictionary access |
| `keydex.Secret` | redacted sensitive value |
| `keydex.KeydexError` | base engine error |
| `keydex.ValidationError` | typed validation failure, `.errors` |
| `keydex.UnknownKeyWarning` | unused key warning |

Every loader accepts `env=` to inject an environment, which is what tests should use
instead of mutating `os.environ`.

## What it does not do yet

- `imports:` / `includes:` between files.
- Remote sources (Vault, S3, Consul) and `watch()`.
- `pydantic`, `attrs` and `msgspec` models (dataclasses only for now).
- Validation ranges/regex (`min`, `max`, patterns).

See [the specification](spec.md) for the agreed design of each one.

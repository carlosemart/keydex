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
  dataclasses, `Enum`, `Path`, `Decimal`, `UUID`, `Secret[str]`.

Known gaps in the type layer, so they do not surprise you:

| Case | What happens today |
|---|---|
| `bool` from a YAML/JSON number (`debug: 1`) | rejected: only the text `"1"`/`"0"` is accepted |
| `Enum` with a wrong value or wrong case | raises `ValueError` from Python, not `ValidationError` |
| `Decimal` from a YAML/JSON number | goes through text and can lose precision (`1.10` → `1.1`) |
| `Secret[T]` with `T != str` | raises `TypeError` at load time |
| `Optional[T]` with a dataclass default | the default is not applied: the field becomes `None` |
| `list[T]` overridden by a profile | **index merge, not replacement** (see the warning below) |

> **Warning — lists merge by index.** With `tags: [a, b]` in the base file and `tags: [c]`
> in a profile, the result is `['c', 'b']`, because the engine stores flat keys
> (`tags[0]`, `tags[1]`). Replacing a list with a shorter one leaves stale entries. This is
> a known defect scheduled to be fixed before 0.1.0 is published; avoid partial list
> overrides until then.

## Environment binding

With a model, the schema disambiguates, so you do not need double underscores:

```bash
APP_DB_HOST=db.internal      # -> db.host
APP_DB_PORT=6000             # -> db.port
APP_DB__HOST=db.internal     # also works, and wins over the relaxed form
```

Without a model (dictionary mode) `__` is mandatory: `APP_DB__HOST`.

Relaxed matching is for **environment variables only**. A `db-host:` key inside a YAML file
does not bind to a field called `db_host`.

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
  "redacted": False,
  "winner":   {"source": "env", "tier": 2, "file": None, "line": None, "col": None},
  "shadowed": [{"source": "file:config/app.yaml", "tier": 4,
                "file": "config/app.yaml", "line": 2, "col": 8}],
  "refs": [],
}
```

Precedence, as implemented: **environment (2) > profile file (3) > base file (4)**, with
defaults coming from the dataclass. The other bands in the specification (CLI arguments,
test overrides, schema defaults) are design, not code.

`explain` exists on `Settings`, not on a typed model: `load` returns your dataclass. If you
need provenance, load in dictionary mode with the same arguments.

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
- Supported: `${key}`, `${key:-default}`, `${env:VAR}`, `${env:VAR:-default}`, nested
  defaults and `\${escaped}`. Relative references (`${.sibling}`) are planned.
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

`repr`, `str` and any log of the dataclass show `***`.

Reading a secret from a file is **opt-in** and stays inside a directory you choose:

```bash
APP_DB__PASSWORD_FILE=/run/secrets/db_password
```

```python
keydex.load(
    App,
    files=["config/app.yaml"],
    env_prefix="APP_",
    secret_files=True,
    secret_root="/run/secrets",
)
```

Without `secret_files=True` nothing is read. With it, a path outside `secret_root` raises
`KeydexError`.

In dictionary mode, declare which keys are secret so diagnostics hide them:

```python
cfg = keydex.load_settings(
    files=["config/app.yaml"],
    env_prefix="APP_",
    sensitive=["db.password"],
)
cfg.explain("db.password")            # value: None, redacted: True
cfg.explain("db.password", reveal=True)["value"]   # the real value
dict(cfg.items())["db.password"]      # None
cfg.get("db.password")                # raw value: get() is the escape hatch
```

`dump` and the CLI use `items()`, so they inherit the redaction.

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

Not every failure is a `KeydexError`: an invalid `Enum` or `Secret[int]` raises `ValueError`
or `TypeError` from the type layer (see the table above).

## CLI

```bash
python -m keydex explain db.host --files config/app.yaml --env-prefix APP_
python -m keydex dump --files config/app.yaml
python -m keydex dump --files config/app.yaml --sensitive db.password
python -m keydex explain db.password --files config/app.yaml --sensitive db.password --reveal
python -m keydex dump --files config/app.yaml --secret-files --secret-root /run/secrets
```

Exit codes: `0` success, `1` engine error, `2` unknown key (argparse also uses `2` for
usage errors). Useful in scripts and containers.

## API summary

| Call | Purpose |
|---|---|
| `keydex.load(Model, *, files, env_prefix, env, profiles, on_unknown, secret_files, secret_root)` | typed load |
| `keydex.load_settings(files, env_prefix, env, profiles, *, sensitive, secret_files, secret_root)` | dictionary mode |
| `Settings.get / [] / in / len / keys / items / explain / refresh` | dictionary access |
| `keydex.Secret` | redacted sensitive value |
| `keydex.KeydexError` | base engine error |
| `keydex.ValidationError` | typed validation failure, `.errors` |
| `keydex.UnknownKeyWarning` | unused key warning |

Every loader accepts `env=` to inject an environment, which is what tests should use
instead of mutating `os.environ`.

## What it does not do yet

- List replacement (index merge today, see the warning above).
- `imports:` / `includes:` between files.
- Remote sources (Vault, S3, Consul) and `watch()`.
- `pydantic`, `attrs` and `msgspec` models (dataclasses only for now).
- Validation ranges, regex or custom validators.
- `explain`/`dump`/`refresh` on a typed model.

See [the specification](spec.md) for the agreed design of each one.

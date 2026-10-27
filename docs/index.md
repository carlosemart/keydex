# keydex

Typed, explainable and fast configuration access for Python, with the engine in Rust.

- **[Python guide](python.md)** — models, profiles, secrets, errors, CLI.
- **[Rust guide](rust.md)** — using `keydex-core` directly, without Python.
- **[API reference](api.md)** — generated from the docstrings.
- **[Specification](spec.md)** — the contract: precedence, binding, IR, budgets.
- **[Architecture decisions](adr/0001-name-keydex.md)** — why each irreversible choice was
  made.

## In one example

```python
from dataclasses import dataclass
import keydex

@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432

@dataclass(frozen=True)
class App:
    db: Db

cfg = keydex.load(App, files=["app.yaml"], env_prefix="APP_")
cfg.db.port        # a real int
```

```bash
APP_DB_HOST=db.internal python app.py   # relaxed binding, no "__" needed
```

Ask why a value is what it is:

```python
raw = keydex.load_settings(files=["app.yaml"], env_prefix="APP_")
raw.explain("db.host")
# {"winner": {"source": "env", "tier": 2, ...},
#  "shadowed": [{"source": "file:app.yaml", "line": 2, "col": 8, ...}]}
```

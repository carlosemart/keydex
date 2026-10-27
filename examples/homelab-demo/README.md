# Demo: homelab service

Exercises keydex's real use case in 60 lines of model:

- **Precedence** between the base file, the profile overlay and the environment.
- **Profiles** that map to environments (`dev`, `prod`).
- **Interpolation** resolved *after* the merge: `dsn` is defined once and in `prod` points
  at `db.internal` without repeating the line.
- **Secrets** that do not leak in `repr`, `str` or the program output, and that can come
  from a file (`*_FILE`).
- **Validation** that stops startup with the exact path of the problem.

## Run

```bash
uv venv && uv pip install -e .          # from the repository root

python examples/homelab-demo/src/main.py --profiles dev
python examples/homelab-demo/src/main.py --profiles prod
```

Output in `prod`:

```
homelab (prod)
  db        postgres://keydex@db.internal:5432/homelab  pool=20
  api       https://api.homelab.local  timeout=10s  retries=2
  backup    /srv/backups  retention=30d
  features  metrics=True tracing=True
  secret    Secret('***') (9 bytes loaded)
```

## Provenance

```bash
python examples/homelab-demo/src/main.py --profiles prod --explain db.host
```

```json
{
  "key": "db.host",
  "value": "db.internal",
  "winner": {"source": "file:.../app-prod.yaml", "tier": 3, "line": 4, "col": 8},
  "shadowed": [
    {"source": "file:.../app.yaml", "tier": 4, "line": 5, "col": 8}
  ],
  "refs": []
}
```

## Overrides and secrets

```bash
APP_DB__HOST=override.local python examples/homelab-demo/src/main.py --profiles prod
APP_SECRET_ROOT=/run/secrets APP_DB__PASSWORD_FILE=/run/secrets/db \
  python examples/homelab-demo/src/main.py --profiles prod
```

Reading from a file is opt-in: the demo enables it only when `APP_SECRET_ROOT` is set, and
a path outside that directory is rejected. If the file does not exist, startup fails with
an explicit error instead of starting with an empty password.

## Invalid configuration

```bash
APP_DB__PORT=not-a-number python examples/homelab-demo/src/main.py --profiles dev
```

```
invalid configuration:
  db.port: expected an integer, got "not-a-number"
```

## Tests

```bash
python -m pytest examples/homelab-demo/tests -q
```

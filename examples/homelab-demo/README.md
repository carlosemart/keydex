# Demo: servicio de homelab

Ejercita el caso de uso real de keydex en 60 líneas de modelo:

- **Precedencia** entre fichero base, overlay de perfil y entorno.
- **Perfiles** que corresponden a entornos (`dev`, `prod`).
- **Interpolación** que se resuelve *después* del merge: `dsn` se define una sola vez y
  en `prod` apunta a `db.internal` sin repetir la línea.
- **Secretos** que no se filtran en `repr`, `str` ni en la salida del programa, y que
  pueden venir de un fichero (`*_FILE`).
- **Validación** que rompe el arranque con la ruta exacta del problema.

## Ejecutar

```bash
uv venv && uv pip install -e .          # desde la raíz del repo

python examples/homelab-demo/src/main.py --profiles dev
python examples/homelab-demo/src/main.py --profiles prod
```

Salida en `prod`:

```
homelab (prod)
  db        postgres://keydex@db.internal:5432/homelab  pool=20
  api       https://api.homelab.local  timeout=10s  reintentos=2
  backup    /srv/backups  retención=30d
  features  metrics=True tracing=True
  secreto   Secret('***') (9 bytes cargados)
```

## Procedencia

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

## Overrides y secretos

```bash
APP_DB__HOST=override.local python examples/homelab-demo/src/main.py --profiles prod
APP_DB__PASSWORD_FILE=/run/secrets/db python examples/homelab-demo/src/main.py --profiles prod
```

Si el fichero de secreto no existe, el arranque falla con un error explícito en vez de
arrancar con una contraseña vacía.

## Configuración inválida

```bash
APP_DB__PORT=no-es-un-numero python examples/homelab-demo/src/main.py --profiles dev
```

```
configuración inválida:
  db.port: se esperaba un entero, llegó "no-es-un-numero"
```

## Tests

```bash
python -m pytest examples/homelab-demo/tests -q
```

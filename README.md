# keydex

Acceso a configuración para Python: **tipado, explicable y rápido**, con el motor en Rust.

> Estado: F1. Motor completo y binding en **modo diccionario** (`keydex.load` +
> `explain`). Los modelos tipados llegan en F2. En PyPI solo hay un placeholder de
> reserva: por ahora se instala desde el repo.

## Instalación (desde el repo)

Requiere Rust estable y [uv](https://docs.astral.sh/uv/).

```bash
uv venv
uv pip install -e .
```

## Uso

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

# Tipado: valida, coacciona y devuelve el modelo
cfg = keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")
cfg.db.port        # int de verdad, no "5432"

# Diccionario: sin modelo, con procedencia
raw = keydex.load_settings(files=["config/app.yaml"], env_prefix="APP_")
raw["db.host"]
raw.explain("db.host")
```

`APP_DB_HOST` casa con `db.host` sin necesidad de `__`: el esquema desambigua.

Los errores se agregan, no se aborta en el primero:

```python
try:
    keydex.load(App, files=["config/app.yaml"], env_prefix="APP_")
except keydex.ValidationError as error:
    error.errors
    # [{"path": "db.port", "message": "se esperaba un entero, llegó \"muchos\""}]
```

El `explain` devuelve una estructura, no un texto:

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

## La idea

```python
cfg = keydex.load(AppConfig, profiles=["prod"])
cfg.db.host                 # valor + tipo correctos, sin FFI por acceso
cfg.explain("db.host")      # de dónde sale, quién lo sombreó y qué placeholders usó
```

Lo que aporta frente a `dynaconf`, `omegaconf` o `pydantic-settings`:

- **Procedencia de primera clase**: cada valor sabe qué fuente, fichero y línea lo
  definió. No es un mensaje de error: es una estructura consultable.
- **Precedencia explícita** por bandas, con inserción relativa de fuentes nuevas.
- **Relaxed binding** resuelto contra el esquema, no por heurísticas.
- **Sin `eval`**: interpolación de referencias y resolvers puros, nada de expresiones.
- **Rendimiento**: el parseo, el merge y la interpolación ocurren una vez en Rust; los
  accesos son Python nativo.

## Qué no es

Un contenedor de inyección de dependencias, un motor de expresiones, un framework de
aplicación, ni una capa de bindings multi-lenguaje. Ver `AGENTS.md`.

## Documentación

- [Especificación](docs/spec.md)
- [Decisiones de arquitectura](docs/adr/)
- [Instrucciones para agentes](AGENTS.md)

## Desarrollo

```bash
./scripts/check.sh     # fmt + clippy
cargo test --workspace --exclude keydex-py
./scripts/gate.sh      # invariantes de arquitectura
```

`scripts/gate.sh` avisa y se salta `cargo deny` si no está instalado. Para tener paridad
con CI: `cargo install cargo-deny --locked`.

## Licencia

MIT OR Apache-2.0, a elección del usuario.

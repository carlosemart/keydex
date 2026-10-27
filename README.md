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
import keydex

cfg = keydex.load(["config/app.yaml"], env_prefix="APP_")

cfg["db.host"]              # valor ya resuelto y con el tipo correcto
cfg.get("db.port", 5432)    # con default
cfg.explain("db.host")      # quién lo definió, quién lo sombreó y qué usó
cfg.keys()
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

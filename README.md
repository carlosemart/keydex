# keydex

Acceso a configuración para Python: **tipado, explicable y rápido**, con el motor en Rust.

> Estado: pre-alpha. El nombre está reservado; todavía no hay nada instalable.

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
cargo test --workspace
./scripts/gate.sh      # invariantes de arquitectura
```

## Licencia

MIT OR Apache-2.0, a elección del usuario.

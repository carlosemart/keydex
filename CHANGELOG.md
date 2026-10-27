# Changelog

Todas las versiones siguen [SemVer](https://semver.org/lang/es/).
El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-27

Primera versión funcional. `0.0.1` fue solo reserva de nombre.

### Added

- **Motor** (`keydex-core`): parsers con spans para YAML, TOML y JSON; precedencia por
  bandas con orden dentro de banda; interpolación `${...}` con defaults, resolvers y
  detección de ciclos; procedencia por valor.
- **Schema IR v1** y modelos tipados: coerción, validación agregada con rutas, binding
  relajado contra el esquema (`APP_DB_HOST` → `db.host` sin `__`), y aviso de claves que
  el esquema no usa (`on_unknown`), con sugerencia por distancia de edición.
- **Perfiles** por convención de ficheros (`app-prod.yaml`).
- **Secretos**: `Secret[T]` con representación redactada e indirección `*_FILE`.
- **`refresh()`** con intercambio atómico del snapshot.
- **CLI**: `python -m keydex explain|dump`.
- **Wheels**: `abi3-py311` (3.11–3.15) y `cp314t` (free-threading real, `gil_used = false`).
- Ejemplo ejecutable [`examples/homelab-demo`](examples/homelab-demo/).
- Gates de arquitectura (`scripts/gate.sh`, `clippy.toml`) y ADRs
  [0006](docs/adr/0006-parsers-con-spans.md) y [0007](docs/adr/0007-schema-ir-con-serde.md).
- Benchmark de carga: 1,26 ms para 1004 claves, 21× más rápido que OmegaConf
  ([detalle](docs/benchmarks.md)).

### Known limitations

- `imports:` / `includes:` sin implementar (ver especificación §9.3).
- Sin fuentes remotas (Vault, S3, Consul) ni `watch()`.

## [0.0.1] - 2026-10-27

Reserva del nombre. Sin implementación funcional.

### Added

- `keydex` 0.0.1 publicado en PyPI como placeholder.
- `keydex-core` 0.0.1 publicado en crates.io como placeholder.

[Unreleased]: https://github.com/carlosemart/keydex/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/carlosemart/keydex/releases/tag/v0.1.0
[0.0.1]: https://github.com/carlosemart/keydex/releases/tag/v0.0.1

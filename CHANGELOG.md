# Changelog

Todas las versiones siguen [SemVer](https://semver.org/lang/es/).
El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/).

## [Unreleased]

### Added

- Esqueleto del workspace Cargo.
- `keydex-core`: `Tier` (bandas de precedencia), `Error` estructurado y
  `SCHEMA_IR_VERSION`.
- Gates de arquitectura (`scripts/gate.sh`) y `clippy.toml` con los métodos
  prohibidos en el core.
- Spike de parsers con spans y [ADR 0006](docs/adr/0006-parsers-con-spans.md):
  `saphyr` (YAML), `toml_edit` (TOML) y `jsonc-parser` (JSON). Cierra F0.
- **F1**: motor completo en `keydex-core` (parsers con spans, precedencia por bandas,
  interpolación `${...}` con `env:` y defaults, `explain` con procedencia) y binding
  PyO3 en modo diccionario (`keydex.load`, `Settings`).
- Benchmark de carga: 1,26 ms para 1004 claves, 21× más rápido que OmegaConf
  ([detalle](docs/benchmarks.md)).
- **F2**: modelos tipados. Schema IR v1 (Python → Rust), coerción y validación
  agregada en el motor, binder relajado contra el esquema (`APP_DB_HOST` → `db.host`
  sin `__`), `keydex.load(Modelo)` y `ValidationError` con `errors` por ruta.
  Convertidores ricos (`Enum`, `Path`, `Decimal`, `UUID`) en la capa de Python.
  ADR [0007](docs/adr/0007-schema-ir-con-serde.md).
- **F3**: perfiles (`app-prod.yaml`), secretos (`Secret[T]`, indirección `*_FILE`),
  `refresh()` con intercambio atómico y CLI (`python -m keydex explain|dump`).
  `includes` queda pendiente para F3.1.
- Ejemplo ejecutable [`examples/homelab-demo`](examples/homelab-demo/): perfiles
  dev/prod, secretos por fichero, interpolación resuelta tras el merge y arranque que
  falla con la ruta exacta si la configuración no valida.

## [0.0.1] - 2026-10-27

Reserva del nombre. Sin implementación funcional.

### Added

- `keydex` 0.0.1 publicado en PyPI como placeholder.
- `keydex-core` 0.0.1 publicado en crates.io como placeholder.

[Unreleased]: https://github.com/carlosemart/keydex/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/carlosemart/keydex/releases/tag/v0.0.1

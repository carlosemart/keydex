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

## [0.0.1] - 2026-10-27

Reserva del nombre. Sin implementación funcional.

### Added

- `keydex` 0.0.1 publicado en PyPI como placeholder.
- `keydex-core` 0.0.1 publicado en crates.io como placeholder.

[Unreleased]: https://github.com/carlosemart/keydex/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/carlosemart/keydex/releases/tag/v0.0.1

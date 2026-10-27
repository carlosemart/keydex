# Changelog

All versions follow [SemVer](https://semver.org/).
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-27

First functional release. `0.0.1` was only a name reservation.

### Added

- **Engine** (`keydex-core`): span-preserving parsers for YAML (including multi-document
  files and `<<: *anchor` merge keys), TOML and JSON; structural merge by precedence tier;
  `${...}` interpolation with defaults, resolvers, nested defaults, escapes and cycle
  detection; per-value provenance and sensitivity.
- **Schema IR v1** and typed models: `str`, `int`, `float`, `bool`, `Optional`, `list`,
  `dict[str, T]`, nested dataclasses, `Enum`, `Path`, `Decimal`, `UUID` and `Secret[T]`;
  coercion and aggregated validation with path, file, line and column; relaxed binding
  against the schema (`APP_DB_HOST` → `db.host`); unknown keys with edit-distance
  suggestions (`on_unknown`).
- **Profiles** through a file naming convention (`app-prod.yaml`).
- **Secrets**: `Secret[T]` marked sensitive in the snapshot; opt-in `*_FILE` confined to
  `secret_root` (the variable is removed from the environment after resolving); redaction
  in `explain`, `items`, `dump`, `settings[key]` and `dict(settings)`; `get()` is the raw
  escape hatch.
- **`Environment`**: one build returns the typed model and keeps the snapshot used by
  `explain`, `settings()` and `refresh()`; `refresh()` re-reads environment and secrets,
  revalidates, and keeps the previous snapshot when the new configuration is invalid.
- **Extension traits** `Source` and `Resolver`; `env` is a built-in resolver. `Format` is
  still a closed enum.
- **Lock-free reads**: the snapshot lives in an `ArcSwap` (ADR 0008).
- **CLI**: `python -m keydex explain|dump`, with `--reveal`, `--sensitive`,
  `--secret-files` and `--secret-root`.
- **Wheels**: `abi3-py311` built; `cp314t` built and tested in CI with the GIL verified
  off. **Nothing is published yet.**
- Runnable example: [`examples/homelab-demo`](examples/homelab-demo/).
- Architecture gates (`scripts/gate.sh` with a self-test, `clippy.toml`, `deny.toml`).

### Changed

- `settings[key]` and `dict(settings)` redact sensitive values, like `items()` and `dump`.
  `get()` remains the raw escape hatch.

### Fixed

- Lists are replaced whole by a higher-priority source; `[]` clears a lower one, and a
  scalar replaces the whole subtree. Literal keys containing dots no longer collide with
  nested paths.
- Sensitivity survives the merge and propagates through interpolation: a value built from
  a secret is redacted too.
- The IR is a validated type (`enum`, `path`, `decimal`, `uuid`, `secret`, `map`), so the
  engine checks enum membership, decimal and UUID lexemes, and rejects a structurally
  invalid IR at parse time.
- `Secret[T]` fields are marked sensitive, so `explain` redacts them instead of printing the
  value.
- `refresh()` re-reads the process environment and the `*_FILE` secrets, and the `*_FILE`
  variable no longer becomes a configuration key.
- `Path` render/parse round-trips keys containing quotes and backslashes; YAML merge keys are
  resolved.
- An `Optional[T]` with a dataclass default keeps that default when the key is absent.

### Documentation

- [Architecture](docs/architecture.md) with a component diagram, module responsibilities,
  load lifecycle, data model and extension seams.
- [Status](docs/status.md) as the single list of what exists and what is missing, plus
  [install](docs/install.md), [troubleshooting](docs/troubleshooting.md),
  [migration](docs/migration.md) and [contributing](docs/contributing.md).

### Performance

- See [benchmarks](docs/benchmarks.md). One dated run, no CI gate.

### Known limitations

- Not published: PyPI holds the `0.0.1` reservation and crates.io the `keydex-core` one.
- `imports:` / `includes:`, remote sources and `watch()` are not implemented.
- `Format` is a closed enum; adding a format means editing the core.
- No validation ranges/regex, no `datetime`/`duration`/`size`, and no
  `pydantic`/`attrs`/`msgspec` models.

## [0.0.1] - 2026-10-27

Name reservation. No functional implementation.

### Added

- `keydex` 0.0.1 published to PyPI as a placeholder.
- `keydex-core` 0.0.1 published to crates.io as a placeholder.

[Unreleased]: https://github.com/carlosemart/keydex/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/carlosemart/keydex/releases/tag/v0.1.0
[0.0.1]: https://github.com/carlosemart/keydex/releases/tag/v0.0.1

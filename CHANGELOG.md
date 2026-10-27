# Changelog

All versions follow [SemVer](https://semver.org/).
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-27

First functional release. `0.0.1` was only a name reservation.

### Added

- **Engine** (`keydex-core`): span-preserving parsers for YAML, TOML and JSON;
  precedence by bands with ordering inside a band; `${...}` interpolation with defaults,
  resolvers and cycle detection; per-value provenance.
- **Schema IR v1** and typed models: coercion, aggregated validation with paths, relaxed
  binding against the schema (`APP_DB_HOST` → `db.host` without `__`), and a warning for
  keys the schema does not use (`on_unknown`), with edit-distance suggestions.
- **Profiles** through a file naming convention (`app-prod.yaml`).
- **Secrets**: `Secret[T]` with a redacted representation, opt-in `*_FILE`
  indirection confined to `secret_root`, and redaction of declared keys in `explain`,
  `items` and `dump`.
- **`refresh()`** with atomic snapshot swap.
- **CLI**: `python -m keydex explain|dump`.
- **Wheels**: `abi3-py311` (3.11–3.15) published; `cp314t` is built and tested in CI
  (`gil_used = false`, GIL verified off) but not published until reads are lock-free.
- Runnable example: [`examples/homelab-demo`](examples/homelab-demo/).
- Architecture gates (`scripts/gate.sh`, `clippy.toml`) and ADRs
  [0006](docs/adr/0006-parsers-with-spans.md) and
  [0007](docs/adr/0007-schema-ir-over-serde.md).
- Load benchmark: 1.26 ms for 1004 keys, 21× faster than OmegaConf
  ([details](docs/benchmarks.md)).

### Fixed

- Lists are replaced whole by a higher-priority source instead of merged by index: a base
  `tags: [a, b]` plus a profile `tags: [c]` now yields `[c]`, and `tags: []` clears it.
  The value model is a tree (`Value::Seq`/`Value::Map`), so a scalar can also replace a
  whole subtree. Literal keys containing dots no longer collide with nested paths.
- Sensitivity is a property of the node: it survives the merge, propagates through
  interpolation (a value built from a secret is redacted too), and drives redaction in
  `explain`, `items`, `dump` and the CLI.
- The Schema IR is a validated type (`enum`, `path`, `decimal`, `uuid`, `secret`), so the
  engine checks enum membership, decimal and UUID lexemes, and rejects a structurally
  invalid IR at parse time instead of returning `null`.
- Validation errors carry the file, line and column of the offending value, and an invalid
  `Enum`/`Decimal`/`UUID` is now a `ValidationError` instead of a raw Python exception.
- An `Optional[T]` with a dataclass default keeps that default when the key is absent.
- `Environment`: one build produces the typed model and the snapshot used by `explain`,
  `settings()` and `refresh()`; the homelab demo no longer loads the configuration twice.
  `refresh()` revalidates and keeps the previous snapshot when the new configuration is
  invalid.

### Added

- `Source` and `Resolver` traits: a caller can add its own source (CLI, Vault, a fixture)
  or its own `${name:args}` resolver without touching the core. `env` is now a built-in
  resolver rather than a special case.
- Lock-free reads: the snapshot lives in an `ArcSwap`, so `get`/`explain` take no lock and
  `refresh()` swaps the pointer (ADR 0008).
- The benchmark now measures the typed path too (200 fields, 1.23 ms).

### Documentation

- New [architecture](docs/architecture.md) page: component diagram, module
  responsibilities, load lifecycle, data model, extension seams and cross-cutting concerns.
- New [status](docs/status.md), [install](docs/install.md),
  [troubleshooting](docs/troubleshooting.md), [migration](docs/migration.md) and
  [contributing](docs/contributing.md) pages, so the guides stop duplicating the same facts.

### Fixed

- `Secret[T]` fields are marked sensitive in the snapshot, so `explain` redacts them instead
  of printing the value.
- `refresh()` re-reads the process environment and the `*_FILE` secrets instead of serving
  the ones captured at load time.
- The `*_FILE` variable no longer becomes a configuration key (it exposed the path and
  produced an unknown-key warning).
- `Path` render/parse round-trips keys containing quotes and backslashes.
- YAML merge keys (`<<: *anchor`) are resolved, with explicit keys winning.

### Known limitations

- `imports:` / `includes:` not implemented (see specification §9.3).
- No remote sources (Vault, S3, Consul) and no `watch()`.

## [0.0.1] - 2026-10-27

Name reservation. No functional implementation.

### Added

- `keydex` 0.0.1 published to PyPI as a placeholder.
- `keydex-core` 0.0.1 published to crates.io as a placeholder.

[Unreleased]: https://github.com/carlosemart/keydex/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/carlosemart/keydex/releases/tag/v0.1.0
[0.0.1]: https://github.com/carlosemart/keydex/releases/tag/v0.0.1

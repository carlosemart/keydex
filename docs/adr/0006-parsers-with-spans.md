# 0006 — Parsers with spans: saphyr, toml_edit and jsonc-parser

- Status: accepted
- Date: 2026-10-27
- Evidence: [spike 0001](../spikes/0001-parsers-spans.md)

## Context

Hard rule 6 requires every key exposed in `explain()` to keep its span. Spike 0001 confirms
that all three formats can provide one, at negligible cost, but only with the right
dependencies.

## Decision

| Format | Crate | API |
|---|---|---|
| YAML | `saphyr` 0.3 | `MarkedYaml::load_from_str` → `span.start.line()/col()` |
| TOML | `toml_edit` 0.25 | `Document::parse` + `Item::span()` (bytes) |
| JSON | `jsonc-parser` 0.34 | `parse_to_ast` + `Ranged::range()` (bytes) |

Discarded:

- `serde_yaml` / `serde_yml`: archived and without spans.
- `marked-yaml`: duplicates the parser (`yaml-rust2` 0.10) and `saphyr` already bundles it.
- `jiter`: fast, but without positions.

## Consequences

- Three format crates, each behind its feature flag (`yaml`, `toml`, `json`).
- The core needs one `LineIndex` per file to convert offsets into line/column; without it
  the parse is quadratic.
- TOML and JSON columns are bytes while YAML columns are characters: the value model
  normalises them to characters.
- A golden test with multibyte UTF-8 is required to pin that normalisation.
- `deny.toml` keeps the veto on `serde_yaml`/`serde_yml`, and the CI gate measures with
  default features so optional dependencies such as `jiter`'s `pyo3` do not confuse it.

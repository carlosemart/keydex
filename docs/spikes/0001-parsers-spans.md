# 0001 — Parser spans

- Date: 2026-10-27
- Spike: [`spikes/spans/`](https://github.com/carlosemart/keydex/tree/main/spikes/spans)
- Question: can we get line and column for every key in YAML, TOML and JSON, and at what
  cost?

## Environment

Rust 1.99.0, `--release` build, AMD Ryzen 9 7940HS. Fixture with 1000 flat keys plus 3
nested ones; median of 200 iterations, including parsing **and** a full walk to extract the
paths.

## Correctness

Nested fixture (`db.host` on line 2 or 3 depending on the format). Columns are 0-based.

| Format | Crate | `db.host` | Expected |
|---|---|---|---|
| YAML | `saphyr` 0.3.3 | `2:8` | ✅ |
| TOML | `toml_edit` 0.25.17 | `2:7` | ✅ |
| JSON | `jsonc-parser` 0.34.0 | `3:12` | ✅ |

## Cost (1000 keys)

| Measurement | µs | µs/key |
|---|---|---|
| YAML with spans | 562 | 0.56 |
| YAML without spans (parse only) | 500 | 0.50 |
| TOML with spans | 570 | 0.57 |
| TOML without spans (parse only) | 574 | — |
| JSON with spans | 127 | 0.13 |

Provenance costs **+12% in YAML** and is indistinguishable from noise in TOML. All three
formats stay well below the 5 ms budget for 1000 keys.

## Findings

1. **`saphyr` 0.3 already bundles `marked_yaml`.** It exposes `MarkedYaml { span, data }`,
   `Span { start, end }` and `Marker::line()` (1-based) / `col()` (0-based). The
   `marked-yaml` crate is not needed, and it would drag `yaml-rust2` 0.10 and duplicate the
   parser.
2. **`toml_edit`: `DocumentMut` drops spans.** `Document`/`ImDocument` must be used, with
   `Item::span()` returning **byte** ranges.
3. **`jsonc-parser` is the cheapest of the three**: 1 crate in the dependency closure, byte
   ranges and JSONC support (comments and trailing commas) for free.
4. **`jiter` is discarded**: it exposes no positions. Its `python` feature also pulls `pyo3`
   (it appears in `Cargo.lock` even though it is not compiled; the gate must measure with
   default features, never `--all-features`).
5. **Measured trap.** Converting byte → line/column by scanning from the start of the file
   for every key makes the parse quadratic: 5.8 ms in TOML and 6.2 ms in JSON with 1000
   keys, against 0.57 and 0.13 with a line index and binary search. **Rule for the core:
   one `LineIndex` per file, built once.**
6. **Heterogeneous columns.** `saphyr` reports columns in characters; TOML and JSON in
   bytes. With multibyte UTF-8 they diverge: they must be normalised to characters.

## Dependency weight (full closure)

| Crate | Crates in the closure |
|---|---|
| `saphyr` | 20 |
| `toml_edit` | 9 |
| `jsonc-parser` | 1 |
| `marked-yaml` (discarded) | 15 |

## Limits of this spike

- Almost fully ASCII and flat fixture: a case with accents and emoji is still needed to fix
  the column normalisation.
- YAML multi-document (imports and profiles) and multi-megabyte documents were not
  measured.
- No memory profiling, only time.

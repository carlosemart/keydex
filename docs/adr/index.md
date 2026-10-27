# Decisions

Every irreversible choice, with the reasoning. An ADR is written once and not rewritten:
if a later decision changes it, the new ADR says so.

| # | Decision | Status |
|---|---|---|
| [0001](0001-name-keydex.md) | The name is `keydex` | accepted |
| [0002](0002-no-expressions.md) | Interpolation has no expression language | accepted |
| [0003](0003-wheels-abi3-and-314t.md) | One `abi3-py311` wheel plus a `cp314t` build | accepted |
| [0004](0004-schema-ir-versioning.md) | The Schema IR is versioned apart from the package | accepted |
| [0005](0005-mandatory-spans.md) | Parsers must preserve spans | accepted; JSON part superseded by 0006 |
| [0006](0006-parsers-with-spans.md) | `saphyr`, `toml_edit` and `jsonc-parser`, with evidence | accepted |
| [0007](0007-schema-ir-over-serde.md) | The IR travels as JSON through `serde` | accepted |
| [0008](0008-arc-swap-lock-free-reads.md) | Reads are lock-free with `ArcSwap` | accepted |

The evidence behind the parser choice is in the [spike](../spikes/0001-parsers-spans.md).

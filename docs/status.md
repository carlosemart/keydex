# Status

Single source of truth for what exists and what does not. `✅` means implemented and
covered by tests; `❌` means designed but not built.

## Loading

| Feature | Status |
|---|---|
| YAML, TOML, JSON parsers with exact spans (line + column) | ✅ |
| Multi-document YAML | ✅ (documents merge in order) |
| INI, `.properties`, `.env` | ❌ |
| Precedence: environment > profile file > base file | ✅ |
| Override / CLI / schema-default tiers | ❌ |
| Profiles by file convention (`app-prod.yaml`) | ✅ |
| Per-document profile activation (`activate.on_profile`) | ❌ |
| `imports:` / `includes:` | ❌ |
| File discovery (`search_paths`, `optional:`) | ❌ |

## Values

| Feature | Status |
|---|---|
| Deep map merge, list replacement, empty list clears | ✅ |
| Scalar replaces a whole subtree | ✅ |
| Literal keys with dots (`a["a.b"]`) | ✅ |
| Interpolation `${key}`, `${key:-default}`, nesting, escapes | ✅ |
| `${env:VAR}` and custom resolvers | ✅ |
| Relative references (`${.sibling}`) | ❌ |
| Cycle detection with a trace | ✅ |

## Types

| Feature | Status |
|---|---|
| `str`, `int`, `float`, `bool`, `Optional`, `list`, nested dataclasses | ✅ |
| `Enum` (case-insensitive, validated in the engine) | ✅ |
| `Path`, `Decimal` (text only), `UUID` | ✅ |
| `Secret[T]` | ✅ |
| `dict[str, T]` (free-form maps, also from `APP_MAP__KEY`) | ✅ |
| `datetime`, `duration`, `size`, `rate`, `URL` | ❌ |
| Discriminated unions, `Literal`, `tuple`, `set` | ❌ |
| `pydantic`, `attrs`, `msgspec`, `TypedDict` | ❌ |

## Validation and errors

| Feature | Status |
|---|---|
| Aggregated errors with path, file, line and column | ✅ |
| Required fields and lists | ✅ |
| Enum / decimal / UUID validation in the engine | ✅ |
| Unknown keys with edit-distance suggestions (`on_unknown`) | ✅ |
| Ranges, regex, custom validators | ❌ |
| Exception hierarchy beyond `KeydexError` / `ValidationError` | ❌ |

## Secrets

| Feature | Status |
|---|---|
| `Secret[T]` with redacted `repr`/`str` | ✅ |
| Opt-in `*_FILE` confined to `secret_root` | ✅ |
| Node-level sensitivity, propagated by merge and interpolation | ✅ |
| Redaction in `explain`, `items`, `dump`, CLI | ✅ |
| Sensitivity carried from the IR into dictionary mode | ❌ |
| `zeroize` of Rust buffers | ❌ |
| Remote secret stores (Vault, keyring, 1Password, SOPS) | ❌ |

## Runtime

| Feature | Status |
|---|---|
| One build, two views (`Environment`) | ✅ |
| `refresh()` with revalidation and atomic swap | ✅ |
| Lock-free reads (`ArcSwap`) | ✅ |
| `watch()` / hot reload | ❌ |
| CLI `explain`, `dump` | ✅ |
| CLI `validate`, `schema` | ❌ |

## Packaging

| Feature | Status |
|---|---|
| `abi3-py311` wheel | ✅ built |
| `cp314t` wheel with the GIL verified off | ✅ built and tested |
| Published to PyPI | ❌ not yet |
| `keydex-core` published to crates.io | ❌ not yet (only the `0.0.1` reservation) |

See the [roadmap](spec.md#21-roadmap) for the order these arrive in.

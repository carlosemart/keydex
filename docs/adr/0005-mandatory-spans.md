# 0005 — Parsers must preserve spans

- Status: accepted
- Date: 2026-10-27

## Context

`explain()` promises to say in which file and on which line a value was defined. Most Rust
parsers discard that information while deserialising.

## Decision

The core parsers must produce values with a span (line and column). `serde_yaml` is
archived and does not keep them, so it is banned: YAML through `saphyr`/`marked_yaml`, TOML
through `toml_edit`, JSON with positions (`jiter` or own indices).

## Consequences

- Parser dependency selection is driven by provenance, not convenience.
- `deny.toml` bans `serde_yaml` and `serde_yml`.
- Fallback if a format exposes no spans: `null` span for that format, documented, rather
  than giving up the others.

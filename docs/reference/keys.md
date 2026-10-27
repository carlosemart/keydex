# Key and path syntax

How a configuration key becomes a path, and how to write paths in interpolation and lookups.

## Canonical form

A path is a list of segments, not a string:

```
db.host                  key "host" inside key "db"
servers[0].host          key "host" inside the first element of "servers"
labels.team_a            key "team_a" inside the map "labels"
```

- `.` separates object levels.
- `[n]` selects a sequence index (0-based).
- Keys are case-sensitive in files; environment variables are matched case-insensitively.

## Literal keys

A key that contains a dot, a bracket or a quote is written in brackets inside a path:

```yaml
"a.b": literal      # the key is the single string a.b
a:
  b: nested         # the path a -> b
```

```python
cfg["a.b"]          # nested
cfg['["a.b"]']      # the literal key
```

Inside brackets, `"` and `\` are escaped with a backslash. Rendering a path quotes such keys
automatically, so `keys()` always returns something that can be passed back to `get()`.

## Environment variables

The variable name is the path with `.` replaced by `_`, after the prefix:

| Variable (`APP_` prefix) | Path | Notes |
|---|---|---|
| `APP_DB_HOST` | `db.host` | typed mode: the schema disambiguates |
| `APP_DB__HOST` | `db.host` | always unambiguous |
| `APP_SERVERS__0__HOST` | `servers[0].host` | explicit level and index |
| `APP_LABELS__TEAM_A` | `labels.team_a` | free-form map entry |

Without a schema (dictionary mode) `__` is the only level separator and `_` is literal.

## Precedence, in one place

```
environment (tier 2)  >  profile file (tier 3)  >  base file (tier 4)  >  code default
```

Inside a tier, files are applied in the order given; the last one wins. The full model,
including the tiers that are design but not implemented, is in
[the specification](../spec.md#4-precedence).

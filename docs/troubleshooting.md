# Troubleshooting

Each entry is a symptom, the cause and the fix. If something is missing here, open an issue
with the output of `python -m keydex dump` (redacted).

## My environment variable does not apply

1. **Prefix.** With `env_prefix="APP_"`, only variables starting with `APP_` are read. A
   bare `DB_HOST` is ignored.
2. **Mode.** With a model, `APP_DB_HOST` binds to `db.host` (the schema disambiguates).
   Without a model, the separator must be `__`: `APP_DB__HOST`.
3. **Case.** Variables are matched case-insensitively, but `_` is literal in dictionary
   mode: `APP_DB_HOST` there means the key `db_host`, not `db.host`.
4. **Ambiguity.** If two schema paths normalise to the same variable, loading fails with
   `AmbiguousBinding` and lists both candidates. Rename one field or use `__`.
5. **Precedence.** The environment beats profiles and base files. If the value does not
   change, the variable is not being read at all: check with
   `python -m keydex explain db.host --env-prefix APP_ --files app.yaml`.

## `*_FILE` does nothing

It is opt-in and confined:

```python
keydex.load(App, files=["app.yaml"], secret_files=True, secret_root="/run/secrets")
```

Without `secret_files=True` no file is read (by design: it would be an arbitrary file-read
surface). A path outside `secret_root`, or a missing file, raises `KeydexError`.

## `UnknownKeyWarning: … unknown key`

A key in the configuration is not used by the model. Almost always a typo or a level nested
wrong: the suggestion tells you which. In CI, make it fatal:

```python
keydex.load(App, files=[...], on_unknown="error")
```

## `explain()` returns `None`

- The key does not exist (typos are not guessed).
- You are on a typed model: a dataclass has no provenance. Use
  `keydex.Environment(...)`, call `load(App)` and then `env.explain(...)`.

## `explain()` shows `value: null, redacted: true`

The key is marked sensitive, either because you listed it in `sensitive=[...]`, because it
came from a `*_FILE` variable, or because it was built from a sensitive value through
interpolation. Use `reveal=True` (`--reveal` in the CLI) when you really need it.

## `get()` returns the secret but the dump does not

That is the contract: `get()`/`[]` are the raw escape hatch; `explain`, `items` and `dump`
are the diagnostic surface and redact. Do not log the result of `get()`.

## A list kept the old values

Fixed in `0.1.0`: a higher-priority source replaces the whole list. If you see stale items,
you are on an older build or the values come from a lower tier that also defines them under
a different key. Check with `python -m keydex dump --files base.yaml --files prod.yaml`.

## `Decimal` was rejected

Decimals must be written as text in the configuration (`price: "1.10"`). A YAML/JSON number
has already lost precision by the time the engine sees it, so it is refused instead of
silently rounding.

## `bool` from `1` was rejected

`debug: 1` in YAML is an integer, not a boolean. Write `debug: true`, or `debug: "1"` if you
insist on the text form.

## `refresh()` changed the snapshot but my object did not

A typed model is a copy. `refresh()` updates the snapshot used by `explain`; call
`env.load(App)` again for a new model.

## `refresh()` raised

The new configuration is invalid and the previous snapshot is still serving. The exception
carries the same `errors` list as a normal load; fix it and call `refresh()` again.

## The build is green but the gate complains

`./scripts/gate.sh` also runs `cargo deny` when installed. The error text names the crate
and the rule (licences, banned crates, sources).

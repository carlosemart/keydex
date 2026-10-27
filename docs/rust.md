# Using keydex-core from Rust

`keydex-core` is the engine on its own: span-preserving parsers, precedence, merge,
interpolation, coercion and provenance. It knows nothing about Python.

```bash
cargo add keydex-core
```

## The two rules that shape the API

1. **The core does not read the environment, the clock or the filesystem.** You read the
   files and capture the environment; the core receives them.
2. **Access is cheap, building is the expensive part.** `build()` produces an immutable
   snapshot; reading from it is a map lookup.

## Dictionary mode

```rust
use std::collections::BTreeMap;
use keydex_core::{Builder, Format, Tier};

let environment: BTreeMap<String, String> = std::iter::once((
    String::from("APP_DB__HOST"),
    String::from("env.internal"),
))
.collect();

let mut builder = Builder::new().with_env(environment);
builder.env_source("APP_", Tier::Environment);
builder.document("app.yaml", yaml_source, Format::Yaml, Tier::BaseFile)?;

let snapshot = builder.build()?;

snapshot.get("db.host");          // Option<&Value>
snapshot.iter();                  // ("key", &Value) in key order
snapshot.keys_with_prefix("db."); // for rebuilding lists
```

`env_source` uses dictionary-mode rules: `__` separates levels and `_` is literal. `Tier`
decides precedence; see [the specification](spec.md#4-precedence).

A complete program: [`examples/basic.rs`](https://github.com/carlosemart/keydex/blob/main/crates/keydex-core/examples/basic.rs)
(`cargo run -p keydex-core --example basic`).

## Schema mode

When you have a model, the IR lets the engine do binding, coercion and validation:

```rust
use keydex_core::{Builder, Schema, Tier, coerce, unknown_keys};

let schema = Schema::from_json(SCHEMA_JSON)?;

let mut builder = Builder::new().with_env(environment);
builder.schema_env_source(&schema, "APP_", Tier::Environment)?;
builder.document("app.yaml", yaml_source, Format::Yaml, Tier::BaseFile)?;
let snapshot = builder.build()?;

let tree: serde_json::Value = match coerce(&snapshot, &schema) {
    Ok(tree) => tree,
    Err(errors) => {
        for error in &errors {
            eprintln!("{}: {}", error.path, error.message);
        }
        return Ok(());
    }
};

for key in unknown_keys(&snapshot, &schema) {
    eprintln!("unused key {} (suggestion: {:?})", key.path, key.suggestion);
}
```

Differences from dictionary mode:

- `APP_DB_HOST` binds to `db.host` **without** `__`, because the schema disambiguates. An
  exact match (`APP_DB__HOST`) still wins.
- Values are coerced to the declared types; errors are aggregated and carry the path.
- Fields that are missing and not required are omitted from the tree, so the caller applies
  its own defaults.

A complete program: [`examples/schema.rs`](https://github.com/carlosemart/keydex/blob/main/crates/keydex-core/examples/schema.rs)
(`cargo run -p keydex-core --example schema`).

The IR is a versioned contract (`keydex.schema/v1`) independent of the crate version. See
[the specification §11](spec.md#11-schema-ir-compatibility-contract).

## Provenance

```rust
if let Some(explanation) = snapshot.explain("db.host") {
    explanation.winner.source;        // "env" or "file:app.yaml"
    explanation.winner.tier;          // Tier::Environment
    explanation.winner.span;          // Option<Span { file, line, col }>
    explanation.shadowed;             // every origin that lost, oldest first
    explanation.refs;                 // keys used through interpolation
}
```

`shadowed` is what answers "why is this value not what I wrote in the file?".

## Errors

One enum, always structured:

```rust
match builder.build() {
    Ok(snapshot) => { /* ... */ }
    Err(keydex_core::Error::MissingReference { path, reference }) => { /* ... */ }
    Err(error) => eprintln!("{error}"),
}
```

`Error` is `#[non_exhaustive]`: match what you handle and keep a catch-all arm.

## Feature flags

| Feature | Default | Brings |
|---|---|---|
| `yaml` | yes | `saphyr` |
| `toml` | yes | `toml_edit` |
| `json` | yes | `jsonc-parser` |
| `schema` | yes | `serde`, `serde_json` |

Disable what you do not use:

```toml
keydex-core = { version = "0.1", default-features = false, features = ["toml"] }
```

## Performance notes

- Parsing, merging and interpolation happen once, in `build()`.
- `LineIndex` converts byte offsets into line/column in O(log n); it is built once per file.
- The snapshot is immutable and `Sync`: share it between threads without locks.

## What it does not do

- No file discovery, no watches, no network. Those belong to the caller.
- No `eval`, no expressions in interpolation.
- No global registration: everything is passed explicitly.

See [the specification](spec.md) for the full contract and
[the ADRs](adr/0001-name-keydex.md) for the reasoning behind each irreversible decision.

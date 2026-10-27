# Using keydex-core from Rust

`keydex-core` is the engine on its own: span-preserving parsers, precedence, merge,
interpolation, coercion and provenance. It knows nothing about Python.

`keydex-core` 0.1.0 is **not published yet** (only the 0.0.1 name reservation exists on
crates.io), so depend on the repository for now:

```toml
[dependencies]
keydex-core = { git = "https://github.com/carlosemart/keydex", branch = "main" }
```

Once 0.1.0 is on crates.io it becomes `cargo add keydex-core`.

## The two rules that shape the API

1. **The core does not read the environment, the clock or the filesystem.** You read the
   files and capture the environment; the core receives them.
2. **Access is cheap, building is the expensive part.** `build()` produces an immutable
   snapshot; reading from it is a map lookup.

## Dictionary mode

This is a complete program (`cargo run -p keydex-core --example basic` runs it):

```rust
use std::collections::BTreeMap;
use keydex_core::{Builder, Format, Tier};

const APP_YAML: &str = "\
db:
  host: file.internal
  port: 5432
  url: \"postgres://${db.host}:${db.port}/app\"
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let environment: BTreeMap<String, String> = std::iter::once((
        String::from("APP_DB__HOST"),
        String::from("env.internal"),
    ))
    .collect();

    let mut builder = Builder::new().with_env(environment);
    builder.env_source("APP_", Tier::Environment);
    builder.document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)?;

    let snapshot = builder.build()?;

    println!("{}", snapshot.get("db.url").unwrap());   // env.internal:5432
    for (key, value) in snapshot.iter() {
        println!("{key} = {value}");
    }

    Ok(())
}
```

`env_source` uses dictionary-mode rules: `__` separates levels and `_` is literal. `Tier`
decides precedence; see [the specification](spec.md#4-precedence).

## Schema mode

This is a complete program (`cargo run -p keydex-core --example schema` runs it):

```rust
use std::collections::BTreeMap;
use keydex_core::{Builder, Format, Schema, Tier, coerce};

const APP_YAML: &str = "db:\n  host: db.internal\n  port: 5432\n";

// What the Python layer generates for a dataclass; hand-written here for the example.
const SCHEMA: &str = r#"{
  "ir": "keydex.schema/v1",
  "root": {"type":"struct","required":true,"fields":{
    "db": {"type":"struct","required":true,"fields":{
      "host": {"type":"str","required":true},
      "port": {"type":"int","required":false}
    }}
  }}
}"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_json(SCHEMA)?;

    let environment: BTreeMap<String, String> =
        std::iter::once((String::from("APP_DB_HOST"), String::from("env.internal")))
            .collect();

    let mut builder = Builder::new().with_env(environment);
    builder.schema_env_source(&schema, "APP_", Tier::Environment)?;
    builder.document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)?;

    let snapshot = builder.build()?;
    let tree = coerce(&snapshot, &schema).map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    println!("{tree:#}");
    Ok(())
}
```

Differences from dictionary mode:

- `APP_DB_HOST` binds to `db.host` **without** `__`, because the schema disambiguates. An
  exact match (`APP_DB__HOST`) still wins.
- Values are coerced to the declared types; errors are aggregated and carry the path.
- Fields that are missing and not required are omitted from the tree, so the caller applies
  its own defaults.

The IR is a versioned contract (`keydex.schema/v1`) independent of the crate version. See
[the specification §11](spec.md#11-schema-ir-compatibility-contract).

## Provenance

```rust
if let Some(explanation) = snapshot.explain("db.host") {
    println!("{}", explanation.winner.source);   // "env" or "file:app.yaml"
    println!("{:?}", explanation.winner.tier);   // Tier::Environment
    for shadowed in &explanation.shadowed {
        if let Some(span) = &shadowed.span {
            println!("shadowed by {}:{}:{}", span.file, span.line, span.col);
        }
    }
}
```

`shadowed` is what answers "why is this value not what I wrote in the file?".

## Errors

One enum, always structured:

```rust
match builder.build() {
    Ok(snapshot) => { /* ... */ }
    Err(keydex_core::Error::MissingReference { path, reference }) => {
        eprintln!("{path} refers to {reference}, which does not exist");
    }
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
- TOML and JSON produce byte offsets, converted to line/column through a `LineIndex` built
  once per file. YAML parsers report the position directly.
- The snapshot is immutable and `Sync`: share it between threads without locks.

## Extending it

Two traits make the engine open without touching it:

```rust
use std::collections::BTreeMap;
use keydex_core::{Builder, Node, Source, SourceId, Tier, Value};

struct CliSource;

impl Source for CliSource {
    fn id(&self) -> SourceId {
        SourceId { name: "cli".into(), tier: Tier::Cli, order: 0 }
    }

    fn read(self: Box<Self>) -> keydex_core::Result<Node> {
        let mut entries = BTreeMap::new();
        entries.insert("debug".into(), Node::new(Value::Bool(true), None));
        Ok(Node::new(Value::Map(entries), None))
    }
}

let mut builder = Builder::new();
builder.source(Box::new(CliSource));
```

Provenance is the builder's job: it stamps every tree with the source id, so a custom
source does not have to know about spans or tiers beyond its own.

A resolver is a pure function behind `${name:args}`:

```rust
struct Upper;

impl keydex_core::Resolver for Upper {
    fn name(&self) -> &'static str { "upper" }
    fn resolve(&self, args: &[&str], _env: &BTreeMap<String, String>)
        -> Result<Value, String> {
        Ok(Value::str(args[0].to_uppercase()))
    }
}

builder.resolver(std::sync::Arc::new(Upper));   // ${upper:hello} -> HELLO
```

`env` is a built-in resolver, registered by default. An unregistered name is a
`UnknownResolver` error, not a silent empty value.

Formats are still a closed enum (`yaml`, `toml`, `json`): adding one means editing the
core. It is the next extension point, not a current one.

## What the core does not do

- No file discovery, no profiles, no watches, no network. Today the Python layer adds
  environment, base files and profile files; a Rust caller registers documents itself.
- No `eval`, no expressions in interpolation.
- No global registration: everything is passed explicitly.

See [the specification](spec.md) for the full contract and
[the ADRs](adr/0001-name-keydex.md) for the reasoning behind each irreversible decision.

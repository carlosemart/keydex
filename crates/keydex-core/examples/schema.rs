//! Schema mode: validate and coerce against a model description (Schema IR).
//!
//!     cargo run -p keydex-core --example schema

use std::collections::BTreeMap;

use keydex_core::{Builder, Format, Schema, Tier, coerce, unknown_keys};

/// What a dataclass in the Python layer would generate for:
///
/// ```text
/// App { db: Db { host: str, port: int = 5432 }, debug: bool = false }
/// ```
const SCHEMA: &str = r#"{
  "ir": "keydex.schema/v1",
  "root": {
    "type": "struct",
    "required": true,
    "fields": {
      "db": {
        "type": "struct",
        "required": true,
        "fields": {
          "host": { "type": "str", "required": true },
          "port": { "type": "int", "required": false }
        }
      },
      "debug": { "type": "bool", "required": false }
    }
  }
}"#;

const APP_YAML: &str = "\
db:
  host: db.internal
  port: 5432
debug: yes
extra: ignored
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_json(SCHEMA)?;

    let environment: BTreeMap<String, String> =
        std::iter::once((String::from("APP_DEBUG"), String::from("no"))).collect();

    let mut builder = Builder::new().with_env(environment);
    // Typed mode: relaxed binding, so `APP_DB_HOST` would match `db.host` too.
    builder.schema_env_source(&schema, "APP_", Tier::Environment)?;
    builder.document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)?;

    let snapshot = builder.build()?;

    match coerce(&snapshot, &schema) {
        Ok(tree) => println!("typed tree: {tree:#}"),
        Err(errors) => {
            // Errors are aggregated, not fail-fast.
            for error in &errors {
                eprintln!("{error}");
            }
            return Ok(());
        }
    }

    for key in unknown_keys(&snapshot, &schema) {
        match key.suggestion {
            Some(suggestion) => println!("unknown key {}: did you mean {suggestion}?", key.path),
            None => println!("unknown key {}", key.path),
        }
    }

    Ok(())
}

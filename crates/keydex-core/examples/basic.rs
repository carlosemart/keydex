//! Dictionary mode: load YAML + environment, resolve interpolation and inspect provenance.
//!
//!     cargo run -p keydex-core --example basic

use std::collections::BTreeMap;

use keydex_core::{Builder, Format, Tier};

const APP_YAML: &str = "\
db:
  host: file.internal
  port: 5432
  url: \"postgres://${db.host}:${db.port}/homelab\"
api:
  base_url: https://api.homelab.local
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The core never reads the process environment: it receives a snapshot of it.
    let environment: BTreeMap<String, String> = [
        ("APP_DB__HOST", "env.internal"),
        ("APP_API__BASE_URL", "https://api.override"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();

    let mut builder = Builder::new().with_env(environment);
    builder.env_source("APP_", Tier::Environment);
    builder.document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)?;

    let snapshot = builder.build()?;

    println!("{} keys", snapshot.len());
    for (key, value) in snapshot.iter() {
        println!("  {key} = {value}");
    }

    println!("\nprovenance of db.host:");
    match snapshot.explain("db.host") {
        Some(explanation) => {
            println!(
                "  winner: {} (tier {})",
                explanation.winner.source,
                explanation.winner.tier.rank()
            );
            for shadowed in &explanation.shadowed {
                match &shadowed.span {
                    Some(span) => {
                        println!("  shadowed by {}:{}:{}", span.file, span.line, span.col)
                    }
                    None => println!("  shadowed by {}", shadowed.source),
                }
            }
            if !explanation.refs.is_empty() {
                println!("  references: {:?}", explanation.refs);
            }
        }
        None => println!("  unknown key"),
    }

    Ok(())
}

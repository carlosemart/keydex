//! End-to-end engine tests: precedence, interpolation and provenance.

use std::collections::BTreeMap;

use keydex_core::{Builder, Error, Format, Tier, Value};

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

const APP_YAML: &str = "\
db:
  host: file.internal
  port: 5432
  url: \"${db.host}:${db.port}\"
name: \"${env:APP_NAME}\"
escaped: '\\${not.a.ref}'
fallback: \"${missing.key:-by-default}\"
";

fn snapshot() -> keydex_core::Snapshot {
    let mut builder = Builder::new().with_env(env(&[
        ("APP_DB__HOST", "env.internal"),
        ("APP_NAME", "service"),
        ("OTHER", "ignored"),
    ]));
    builder.env_source("APP_", Tier::Environment);
    builder
        .document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)
        .unwrap();
    builder.build().unwrap()
}

#[test]
fn environment_wins_over_file() {
    let cfg = snapshot();
    assert_eq!(cfg.get("db.host"), Some(&Value::str("env.internal")));
    assert_eq!(cfg.get("db.port"), Some(&Value::Int(5432)));
}

#[test]
fn explain_reports_winner_and_shadowed() {
    let cfg = snapshot();
    let explanation = cfg.explain("db.host").unwrap();

    assert_eq!(explanation.winner.source, "env");
    assert_eq!(explanation.winner.tier, Tier::Environment);

    assert_eq!(explanation.shadowed.len(), 1);
    let shadowed = &explanation.shadowed[0];
    assert_eq!(shadowed.source, "file:app.yaml");
    let span = shadowed.span.as_ref().unwrap();
    assert_eq!(span.line, 2);
    assert_eq!(span.col, 8);
}

#[test]
fn interpolation_keeps_the_type() {
    // `${db.port}` is an integer inside a string: it is substituted by its text.
    let cfg = snapshot();
    assert_eq!(cfg.get("db.url"), Some(&Value::str("env.internal:5432")));
}

#[test]
fn resolves_the_env_resolver() {
    let cfg = snapshot();
    assert_eq!(cfg.get("name"), Some(&Value::str("service")));
}

#[test]
fn escaped_placeholders_stay_literal() {
    let cfg = snapshot();
    assert_eq!(cfg.get("escaped"), Some(&Value::str("${not.a.ref}")));
}

#[test]
fn uses_the_default_when_the_key_is_missing() {
    let cfg = snapshot();
    assert_eq!(cfg.get("fallback"), Some(&Value::str("by-default")));
}

#[test]
fn a_missing_reference_without_default_is_an_error() {
    let mut builder = Builder::new();
    builder
        .document(
            "broken.yaml",
            "a: \"${does.not.exist}\"\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let error = builder.build().unwrap_err();
    assert!(matches!(
        error,
        Error::MissingReference { ref reference, .. } if reference == "does.not.exist"
    ));
}

#[test]
fn detects_cycles_with_a_trace() {
    let mut builder = Builder::new();
    builder
        .document(
            "cycle.yaml",
            "a: \"${b}\"\nb: \"${a}\"\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let error = builder.build().unwrap_err();
    match error {
        Error::InterpolationCycle { cycle } => {
            assert!(cycle.contains('a'), "trace: {cycle}");
            assert!(cycle.contains('b'), "trace: {cycle}");
        }
        other => panic!("expected InterpolationCycle, got {other:?}"),
    }
}

#[test]
fn explain_records_the_references_used() {
    let cfg = snapshot();
    let explanation = cfg.explain("db.url").unwrap();
    assert!(explanation.refs.contains(&"db.host".to_string()));
    assert!(explanation.refs.contains(&"db.port".to_string()));
}

#[test]
fn toml_keeps_spans() {
    let mut builder = Builder::new();
    builder
        .document(
            "app.toml",
            "[db]\nhost = \"db.internal\"\nport = 5432\n",
            Format::Toml,
            Tier::BaseFile,
        )
        .unwrap();
    let cfg = builder.build().unwrap();

    let explanation = cfg.explain("db.host").unwrap();
    let span = explanation.winner.span.unwrap();
    assert_eq!((span.line, span.col), (2, 7));
    assert_eq!(cfg.get("db.port"), Some(&Value::Int(5432)));
}

#[test]
fn json_keeps_spans_and_does_not_turn_integers_into_floats() {
    let mut builder = Builder::new();
    builder
        .document(
            "app.json",
            "{\n  \"db\": {\n    \"host\": \"db.internal\",\n    \"port\": 5432\n  }\n}\n",
            Format::Json,
            Tier::BaseFile,
        )
        .unwrap();
    let cfg = builder.build().unwrap();

    let explanation = cfg.explain("db.host").unwrap();
    let span = explanation.winner.span.unwrap();
    assert_eq!((span.line, span.col), (3, 12));
    assert_eq!(cfg.get("db.port"), Some(&Value::Int(5432)));
}

#[test]
fn lists_are_flattened_with_indices() {
    let mut builder = Builder::new();
    builder
        .document(
            "list.yaml",
            "servers:\n  - host: a\n  - host: b\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let cfg = builder.build().unwrap();
    assert_eq!(cfg.get("servers[0].host"), Some(&Value::str("a")));
    assert_eq!(cfg.get("servers[1].host"), Some(&Value::str("b")));
}

#[test]
fn source_order_inside_a_band_decides() {
    let mut builder = Builder::new();
    builder
        .document("a.yaml", "k: first\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    builder
        .document("b.yaml", "k: second\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let cfg = builder.build().unwrap();

    assert_eq!(cfg.get("k"), Some(&Value::str("second")));
    let explanation = cfg.explain("k").unwrap();
    assert_eq!(explanation.shadowed.len(), 1);
    assert_eq!(explanation.shadowed[0].source, "file:a.yaml");
}

#[test]
fn the_build_is_deterministic() {
    let a: Vec<String> = snapshot().keys().map(ToString::to_string).collect();
    let b: Vec<String> = snapshot().keys().map(ToString::to_string).collect();
    assert_eq!(a, b);
    assert!(a.windows(2).all(|w| w[0] <= w[1]), "keys not sorted");
}

#[test]
fn keys_without_the_prefix_are_ignored() {
    let cfg = snapshot();
    assert!(cfg.get("other").is_none());
}

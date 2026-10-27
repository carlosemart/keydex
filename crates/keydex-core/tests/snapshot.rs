//! Tests de extremo a extremo del motor: precedencia, interpolación y procedencia.

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
fallback: \"${missing.key:-por-defecto}\"
";

fn snapshot() -> keydex_core::Snapshot {
    let mut builder = Builder::new().with_env(env(&[
        ("APP_DB__HOST", "env.internal"),
        ("APP_NAME", "servicio"),
        ("OTRA", "ignorada"),
    ]));
    builder.env_source("APP_", Tier::Environment);
    builder
        .document("app.yaml", APP_YAML, Format::Yaml, Tier::BaseFile)
        .unwrap();
    builder.build().unwrap()
}

#[test]
fn el_entorno_gana_al_fichero() {
    let cfg = snapshot();
    assert_eq!(cfg.get("db.host"), Some(&Value::str("env.internal")));
    assert_eq!(cfg.get("db.port"), Some(&Value::Int(5432)));
}

#[test]
fn explain_dice_quien_gano_y_quien_perdio() {
    let cfg = snapshot();
    let explanation = cfg.explain("db.host").unwrap();

    assert_eq!(explanation.winner.source, "env");
    assert_eq!(explanation.winner.tier, Tier::Environment);

    assert_eq!(explanation.shadowed.len(), 1);
    let sombreado = &explanation.shadowed[0];
    assert_eq!(sombreado.source, "file:app.yaml");
    let span = sombreado.span.as_ref().unwrap();
    assert_eq!(span.line, 2);
    assert_eq!(span.col, 8);
}

#[test]
fn la_interpolacion_hereda_el_tipo() {
    // `${db.port}` es un entero dentro de una cadena: se sustituye por su texto.
    let cfg = snapshot();
    assert_eq!(cfg.get("db.url"), Some(&Value::str("env.internal:5432")));
}

#[test]
fn resuelve_el_resolver_env() {
    let cfg = snapshot();
    assert_eq!(cfg.get("name"), Some(&Value::str("servicio")));
}

#[test]
fn las_placeholders_escapadas_quedan_literales() {
    let cfg = snapshot();
    assert_eq!(cfg.get("escaped"), Some(&Value::str("${not.a.ref}")));
}

#[test]
fn usa_el_default_cuando_falta_la_clave() {
    let cfg = snapshot();
    assert_eq!(cfg.get("fallback"), Some(&Value::str("por-defecto")));
}

#[test]
fn una_referencia_inexistente_sin_default_es_error() {
    let mut builder = Builder::new();
    builder
        .document(
            "rota.yaml",
            "a: \"${no.existe}\"\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let error = builder.build().unwrap_err();
    assert!(matches!(
        error,
        Error::MissingReference { ref reference, .. } if reference == "no.existe"
    ));
}

#[test]
fn detecta_ciclos_con_traza() {
    let mut builder = Builder::new();
    builder
        .document(
            "ciclo.yaml",
            "a: \"${b}\"\nb: \"${a}\"\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let error = builder.build().unwrap_err();
    match error {
        Error::InterpolationCycle { cycle } => {
            assert!(cycle.contains("a"), "traza: {cycle}");
            assert!(cycle.contains("b"), "traza: {cycle}");
        }
        otro => panic!("se esperaba InterpolationCycle, llegó {otro:?}"),
    }
}

#[test]
fn explain_registra_las_referencias_usadas() {
    let cfg = snapshot();
    let explanation = cfg.explain("db.url").unwrap();
    assert!(explanation.refs.contains(&"db.host".to_string()));
    assert!(explanation.refs.contains(&"db.port".to_string()));
}

#[test]
fn toml_conserva_spans() {
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
fn json_conserva_spans_y_no_convierte_enteros_en_flotantes() {
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
fn las_listas_se_aplanan_con_indices() {
    let mut builder = Builder::new();
    builder
        .document(
            "lista.yaml",
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
fn el_orden_de_fuentes_dentro_de_la_banda_decide() {
    let mut builder = Builder::new();
    builder
        .document("a.yaml", "k: primera\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    builder
        .document("b.yaml", "k: segunda\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let cfg = builder.build().unwrap();

    assert_eq!(cfg.get("k"), Some(&Value::str("segunda")));
    let explanation = cfg.explain("k").unwrap();
    assert_eq!(explanation.shadowed.len(), 1);
    assert_eq!(explanation.shadowed[0].source, "file:a.yaml");
}

#[test]
fn el_build_es_determinista() {
    let a: Vec<String> = snapshot().keys().map(ToString::to_string).collect();
    let b: Vec<String> = snapshot().keys().map(ToString::to_string).collect();
    assert_eq!(a, b);
    assert!(a.windows(2).all(|w| w[0] <= w[1]), "claves desordenadas");
}

#[test]
fn las_claves_sin_prefijo_se_ignoran() {
    let cfg = snapshot();
    assert!(cfg.get("otra").is_none());
}

//! Tests del Schema IR: binding relajado, coerción y errores agregados.

use std::collections::BTreeMap;

use keydex_core::{Builder, Format, Schema, Tier, coerce};
use serde_json::json;

const IR: &str = r#"{
  "ir": "keydex.schema/v1",
  "root": {"type":"struct","required":true,"fields":{
     "db":{"type":"struct","required":true,"fields":{
        "host":{"type":"str","required":true},
        "port":{"type":"int","required":false},
        "tls":{"type":"bool","required":false},
        "tags":{"type":"list","required":false,"items":{"type":"str"}}
     }},
     "debug":{"type":"bool","required":false},
     "note":{"type":"optional","required":false,"inner":{"type":"str"}}
  }}
}"#;

const YAML: &str = "\
db:
  host: file.internal
  port: 5432
  tls: true
  tags:
    - alpha
    - beta
";

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn load(ir: &str, yaml: &str, env_pairs: &[(&str, &str)]) -> serde_json::Value {
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(env_pairs));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("app.yaml", yaml, Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();
    coerce(&snapshot, &schema).unwrap()
}

#[test]
fn el_binding_relajado_no_necesita_doble_guion_bajo() {
    let tree = load(IR, YAML, &[("APP_DB_HOST", "env.internal")]);
    assert_eq!(tree["db"]["host"], json!("env.internal"));
    assert_eq!(tree["db"]["port"], json!(5432));
    assert_eq!(tree["db"]["tags"], json!(["alpha", "beta"]));
}

#[test]
fn el_doble_guion_bajo_tambien_funciona() {
    let tree = load(IR, YAML, &[("APP_DB__HOST", "via.doble")]);
    assert_eq!(tree["db"]["host"], json!("via.doble"));
}

#[test]
fn coacciona_booleanos_relajados() {
    let tree = load(IR, YAML, &[("APP_DEBUG", "yes")]);
    assert_eq!(tree["debug"], json!(true));
}

#[test]
fn los_campos_opcionales_ausentes_valen_null() {
    let tree = load(IR, YAML, &[]);
    assert_eq!(tree["note"], json!(null));
}

#[test]
fn los_requeridos_ausentes_son_error_con_ruta() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("vacio.yaml", "debug: true\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();

    let errors = coerce(&snapshot, &schema).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.path == "db.host" && e.message.contains("requerido")),
        "errores: {errors:?}"
    );
}

#[test]
fn un_tipo_incorrecto_es_error_con_ruta_y_valor() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[("APP_DB__PORT", "muchos")]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("app.yaml", "db:\n  host: h\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();

    let errors = coerce(&snapshot, &schema).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.path == "db.port" && e.message.contains("entero")),
        "errores: {errors:?}"
    );
}

#[test]
fn los_errores_se_agregan() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder =
        Builder::new().with_env(env(&[("APP_DB__PORT", "muchos"), ("APP_DEBUG", "quizas")]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "vacio.yaml",
            "db:\n  host: h\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();

    let errors = coerce(&snapshot, &schema).unwrap_err();
    assert_eq!(errors.len(), 2, "errores: {errors:?}");
}

#[test]
fn un_binding_ambiguo_es_error() {
    // `a.b.c` y `a.b_c` colisionan en la forma relajada (`a_b_c`) y ninguna casa de
    // forma exacta con `APP_A_B_C`.
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "a": {"type":"struct","required":false,"fields":{
          "b": {"type":"struct","required":false,"fields":{
            "c": {"type":"str","required":false}
          }},
          "b_c": {"type":"str","required":false}
        }}
      }}
    }"#;
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(&[("APP_A_B_C", "x")]));
    let error = builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap_err();
    assert!(matches!(error, keydex_core::Error::AmbiguousBinding { .. }));
}

#[test]
fn la_coincidencia_exacta_gana_a_la_relajada() {
    // `APP_A_B` casa exactamente con el campo `a_b`, aunque `a.b` también exista.
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "a_b": {"type":"str","required":false},
        "a": {"type":"struct","required":false,"fields":{
          "b": {"type":"str","required":false}
        }}
      }}
    }"#;
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(&[("APP_A_B", "literal")]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("x.yaml", "", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();
    let tree = coerce(&snapshot, &schema).unwrap();
    assert_eq!(tree["a_b"], json!("literal"));
    // `a` no es requerido y quedó vacío: se omite para que Python aplique su default.
    assert!(tree.get("a").is_none(), "árbol: {tree}");
}

#[test]
fn rechaza_un_ir_de_otra_version() {
    let error =
        Schema::from_json(r#"{"ir":"keydex.schema/v2","root":{"type":"str"}}"#).unwrap_err();
    assert!(matches!(
        error,
        keydex_core::Error::UnsupportedFormat { .. }
    ));
}

#[test]
fn detecta_claves_desconocidas_con_sugerencia() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "x.yaml",
            "db:\n  host: h\n  prot: 1\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();

    let unknown = keydex_core::unknown_keys(&snapshot, &schema);
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert_eq!(unknown[0].path, "db.prot");
    assert_eq!(unknown[0].suggestion.as_deref(), Some("db.port"));
}

#[test]
fn sugiere_la_ruta_cuando_el_nivel_esta_mal_anidado() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "name": {"type":"str","required":true},
        "db": {"type":"struct","required":false,"fields":{
          "host": {"type":"str","required":false}
        }}
      }}
    }"#;
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("x.yaml", "app:\n  name: h\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();

    let unknown = keydex_core::unknown_keys(&snapshot, &schema);
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert_eq!(unknown[0].path, "app.name");
    assert_eq!(unknown[0].suggestion.as_deref(), Some("name"));
}

#[test]
fn las_claves_anidadas_dentro_de_listas_son_conocidas() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "servers": {"type":"list","required":false,"items":{
          "type":"struct","required":false,"fields":{
            "host": {"type":"str","required":false}
          }
        }}
      }}
    }"#;
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "x.yaml",
            "servers:\n  - host: a\n  - host: b\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();

    assert!(keydex_core::unknown_keys(&snapshot, &schema).is_empty());
}

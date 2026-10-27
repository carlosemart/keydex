//! Schema IR tests: relaxed binding, coercion and aggregated errors.

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
fn relaxed_binding_does_not_need_double_underscore() {
    let tree = load(IR, YAML, &[("APP_DB_HOST", "env.internal")]);
    assert_eq!(tree["db"]["host"], json!("env.internal"));
    assert_eq!(tree["db"]["port"], json!(5432));
    assert_eq!(tree["db"]["tags"], json!(["alpha", "beta"]));
}

#[test]
fn double_underscore_also_works() {
    let tree = load(IR, YAML, &[("APP_DB__HOST", "via.double")]);
    assert_eq!(tree["db"]["host"], json!("via.double"));
}

#[test]
fn coerces_relaxed_booleans() {
    let tree = load(IR, YAML, &[("APP_DEBUG", "yes")]);
    assert_eq!(tree["debug"], json!(true));
}

#[test]
fn missing_optional_fields_are_null() {
    let tree = load(IR, YAML, &[]);
    assert_eq!(tree["note"], json!(null));
}

#[test]
fn missing_required_fields_are_an_error_with_a_path() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("empty.yaml", "debug: true\n", Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();

    let errors = coerce(&snapshot, &schema).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.path == "db.host" && e.message.contains("required")),
        "errors: {errors:?}"
    );
}

#[test]
fn a_wrong_type_is_an_error_with_path_and_value() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[("APP_DB__PORT", "many")]));
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
            .any(|e| e.path == "db.port" && e.message.contains("integer")),
        "errors: {errors:?}"
    );
}

#[test]
fn errors_are_aggregated() {
    let schema = Schema::from_json(IR).unwrap();
    let mut builder =
        Builder::new().with_env(env(&[("APP_DB__PORT", "many"), ("APP_DEBUG", "maybe")]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "empty.yaml",
            "db:\n  host: h\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();

    let errors = coerce(&snapshot, &schema).unwrap_err();
    assert_eq!(errors.len(), 2, "errors: {errors:?}");
}

#[test]
fn an_ambiguous_binding_is_an_error() {
    // `a.b.c` and `a.b_c` collide in the relaxed form (`a_b_c`) and neither matches
    // `APP_A_B_C` exactly.
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
fn exact_match_beats_relaxed() {
    // `APP_A_B` matches the field `a_b` exactly, even though `a.b` also exists.
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
    // `a` is not required and ended up empty: it is omitted so Python applies its default.
    assert!(tree.get("a").is_none(), "tree: {tree}");
}

#[test]
fn rejects_an_ir_from_another_version() {
    let error =
        Schema::from_json(r#"{"ir":"keydex.schema/v2","root":{"type":"str"}}"#).unwrap_err();
    assert!(matches!(
        error,
        keydex_core::Error::UnsupportedSchema { .. }
    ));
}

#[test]
fn detects_unknown_keys_with_a_suggestion() {
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
fn suggests_the_path_when_the_level_is_nested_wrong() {
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
fn nested_keys_inside_lists_are_known() {
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

fn load_ir(ir: &str, yaml: &str) -> Result<serde_json::Value, Vec<keydex_core::ValidationError>> {
    let schema = Schema::from_json(ir).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document("app.yaml", yaml, Format::Yaml, Tier::BaseFile)
        .unwrap();
    let snapshot = builder.build().unwrap();
    coerce(&snapshot, &schema)
}

const RICH_IR: &str = r#"{
  "ir": "keydex.schema/v1",
  "root": {"type":"struct","required":true,"fields":{
    "mode": {"type":"enum","variants":["fast","slow"],"required":true},
    "price": {"type":"decimal","required":false},
    "id": {"type":"uuid","required":false},
    "note": {"type":"optional","has_default":true,"required":false,"inner":{"type":"str"}}
  }}
}"#;

#[test]
fn enum_matching_is_case_insensitive_and_canonical() {
    let tree = load_ir(RICH_IR, "mode: SLOW\n").unwrap();
    assert_eq!(tree["mode"], json!("slow"));
}

#[test]
fn an_unknown_enum_value_is_a_validation_error() {
    let errors = load_ir(RICH_IR, "mode: turbo\n").unwrap_err();
    assert_eq!(errors[0].path, "mode");
    assert!(errors[0].message.contains("expected one of"), "{errors:?}");
}

#[test]
fn decimals_are_validated_as_text() {
    let tree = load_ir(RICH_IR, "mode: fast\nprice: \"1.10\"\n").unwrap();
    assert_eq!(tree["price"], json!("1.10"));

    let errors = load_ir(RICH_IR, "mode: fast\nprice: \"many\"\n").unwrap_err();
    assert!(errors[0].message.contains("decimal"), "{errors:?}");
}

#[test]
fn a_decimal_from_a_yaml_number_is_rejected_to_keep_precision() {
    let errors = load_ir(RICH_IR, "mode: fast\nprice: 1.10\n").unwrap_err();
    assert!(errors[0].message.contains("as text"), "{errors:?}");
}

#[test]
fn uuids_are_validated() {
    let tree = load_ir(
        RICH_IR,
        "mode: fast\nid: 550E8400-E29B-41D4-A716-446655440000\n",
    )
    .unwrap();
    assert_eq!(tree["id"], json!("550e8400-e29b-41d4-a716-446655440000"));

    let errors = load_ir(RICH_IR, "mode: fast\nid: nope\n").unwrap_err();
    assert!(errors[0].message.contains("UUID"), "{errors:?}");
}

#[test]
fn an_optional_with_a_default_is_omitted_when_missing() {
    let tree = load_ir(RICH_IR, "mode: fast\n").unwrap();
    assert!(tree.get("note").is_none(), "{tree}");
}

#[test]
fn validation_errors_carry_the_span() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "port": {"type":"int","required":true}
      }}
    }"#;
    let errors = load_ir(ir, "port: many\n").unwrap_err();
    let span = errors[0].span.as_ref().expect("span");
    assert_eq!(span.file, "app.yaml");
    assert_eq!(span.line, 1);
}

#[test]
fn a_structurally_invalid_ir_is_rejected_at_parse_time() {
    let error =
        Schema::from_json(r#"{"ir":"keydex.schema/v1","root":{"type":"str"}}"#).unwrap_err();
    assert!(matches!(error, keydex_core::Error::InvalidSchema { .. }));

    let error = Schema::from_json(
        r#"{"ir":"keydex.schema/v1","root":{"type":"struct","fields":{"m":{"type":"enum","variants":[]}}}}"#,
    )
    .unwrap_err();
    assert!(matches!(error, keydex_core::Error::InvalidSchema { .. }));
}

const MAP_IR: &str = r#"{
  "ir": "keydex.schema/v1",
  "root": {"type":"struct","required":true,"fields":{
    "labels": {"type":"map","required":false,"values":{"type":"str","required":false}},
    "limits": {"type":"map","required":false,"values":{"type":"int","required":false}}
  }}
}"#;

#[test]
fn a_map_accepts_free_form_keys() {
    let tree = load_ir(
        MAP_IR,
        "labels:\n  env: prod\n  \"team.a\": core\nlimits:\n  cpu: 2\n",
    )
    .unwrap();

    assert_eq!(tree["labels"]["env"], json!("prod"));
    assert_eq!(tree["labels"]["team.a"], json!("core"));
    assert_eq!(tree["limits"]["cpu"], json!(2));
}

#[test]
fn a_map_binds_environment_keys_with_double_underscore() {
    let schema = Schema::from_json(MAP_IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[("APP_LABELS__TEAM_A", "core")]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "app.yaml",
            "labels:\n  env: prod\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();
    let tree = coerce(&snapshot, &schema).unwrap();

    assert_eq!(tree["labels"]["team_a"], json!("core"));
    assert_eq!(tree["labels"]["env"], json!("prod"));
}

#[test]
fn keys_inside_a_map_are_not_unknown() {
    let schema = Schema::from_json(MAP_IR).unwrap();
    let mut builder = Builder::new().with_env(env(&[]));
    builder
        .schema_env_source(&schema, "APP_", Tier::Environment)
        .unwrap();
    builder
        .document(
            "app.yaml",
            "labels:\n  anything: goes\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    let snapshot = builder.build().unwrap();

    assert!(keydex_core::unknown_keys(&snapshot, &schema).is_empty());
}

#[test]
fn a_bad_map_value_is_a_validation_error() {
    let errors = load_ir(MAP_IR, "limits:\n  cpu: many\n").unwrap_err();
    assert_eq!(errors[0].path, "limits.cpu");
}

#[test]
fn a_required_secret_that_is_absent_is_a_validation_error() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "password": {"type":"secret","required":true,"inner":{"type":"str","required":false}}
      }}
    }"#;
    let errors = load_ir(ir, "other: x\n").unwrap_err();
    assert_eq!(errors[0].path, "password");
    assert!(errors[0].message.contains("required"), "{errors:?}");
}

#[test]
fn a_struct_with_a_default_is_omitted_when_absent() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "db": {"type":"struct","required":false,"has_default":true,"fields":{
          "host": {"type":"str","required":true}
        }}
      }}
    }"#;
    let tree = load_ir(ir, "other: x\n").unwrap();
    assert!(tree.get("db").is_none(), "{tree}");
}

#[test]
fn a_str_field_rejects_a_list_instead_of_stringifying_it() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "name": {"type":"str","required":true}
      }}
    }"#;
    let errors = load_ir(ir, "name:\n  - a\n  - b\n").unwrap_err();
    assert_eq!(errors[0].path, "name");
    assert!(errors[0].message.contains("string"), "{errors:?}");
}

#[test]
fn an_integer_is_not_a_boolean() {
    let ir = r#"{
      "ir": "keydex.schema/v1",
      "root": {"type":"struct","required":true,"fields":{
        "tls": {"type":"bool","required":false}
      }}
    }"#;
    let errors = load_ir(ir, "tls: 1\n").unwrap_err();
    assert_eq!(errors[0].path, "tls");
}

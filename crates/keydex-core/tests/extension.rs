//! Extension points: a custom source and a custom resolver, without touching the core.

use std::collections::BTreeMap;
use std::sync::Arc;

use keydex_core::{Builder, Format, Node, Resolver, Source, SourceId, Tier, Value};

/// A source that pretends to read command line arguments.
struct CliSource;

impl Source for CliSource {
    fn id(&self) -> SourceId {
        SourceId {
            name: String::from("cli"),
            tier: Tier::Cli,
            order: 0,
        }
    }

    fn read(self: Box<Self>) -> keydex_core::Result<Node> {
        let mut entries = BTreeMap::new();
        entries.insert(String::from("k"), Node::new(Value::str("from-cli"), None));
        entries.insert(
            String::from("only_cli"),
            Node::new(Value::str("cli-value"), None),
        );
        Ok(Node::new(Value::Map(entries), None))
    }
}

struct Upper;

impl Resolver for Upper {
    fn name(&self) -> &'static str {
        "upper"
    }

    fn resolve(
        &self,
        args: &[&str],
        _env: &BTreeMap<String, String>,
    ) -> std::result::Result<Value, String> {
        if args.len() != 1 {
            return Err(String::from("upper expects one argument"));
        }
        Ok(Value::str(args[0].to_uppercase()))
    }
}

#[test]
fn a_custom_source_takes_part_in_precedence() {
    let mut builder = Builder::new();
    builder
        .document(
            "app.yaml",
            "k: from-file\nother: kept\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();
    builder.source(Box::new(CliSource));

    let snapshot = builder.build().unwrap();

    assert_eq!(snapshot.get("k"), Some(&Value::str("from-cli")));
    assert_eq!(snapshot.get("only_cli"), Some(&Value::str("cli-value")));
    assert_eq!(snapshot.get("other"), Some(&Value::str("kept")));
    assert_eq!(snapshot.explain("k").unwrap().winner.source, "cli");
}

struct Custom;

impl Source for Custom {
    fn id(&self) -> SourceId {
        SourceId {
            name: String::from("custom"),
            tier: Tier::Override,
            order: 0,
        }
    }

    fn read(self: Box<Self>) -> keydex_core::Result<Node> {
        let mut entries = BTreeMap::new();
        entries.insert(
            String::from("greeting"),
            Node::new(Value::str("${upper:hello}"), None),
        );
        Ok(Node::new(Value::Map(entries), None))
    }
}

#[test]
fn a_custom_resolver_is_available_to_interpolation() {
    let mut builder = Builder::new();
    builder.resolver(Arc::new(Upper));
    builder.source(Box::new(Custom));

    let snapshot = builder.build().unwrap();
    assert_eq!(snapshot.get("greeting"), Some(&Value::str("HELLO")));
}

#[test]
fn an_unknown_resolver_is_an_error() {
    let mut builder = Builder::new();
    builder
        .document(
            "x.yaml",
            "a: \"${nope:value}\"\n",
            Format::Yaml,
            Tier::BaseFile,
        )
        .unwrap();

    let error = builder.build().unwrap_err();
    assert!(matches!(
        error,
        keydex_core::Error::UnknownResolver { ref name, .. } if name == "nope"
    ));
}

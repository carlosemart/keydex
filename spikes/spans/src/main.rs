//! Spike F0: ¿podemos obtener spans (línea/columna) de YAML, TOML y JSON?
//!
//! Mide además el coste de parsear + recorrer 1000 claves por formato, y el coste
//! añadido de conservar procedencia frente a parsear sin ella.

use std::time::Instant;

use jsonc_parser::common::Ranged;

/// Índice de inicios de línea: convierte un offset de bytes en (línea, columna)
/// en O(log n). Sin esto la conversión es O(n) por clave y el parseo se vuelve
/// cuadrático (medido: 10× más lento con 1000 claves).
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(src: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.starts.partition_point(|&s| s <= offset).max(1);
        (line, offset - self.starts[line - 1])
    }
}

// ---------------------------------------------------------------- YAML (saphyr)

fn yaml_key<'a>(d: &saphyr::YamlData<'a, saphyr::MarkedYaml<'a>>) -> String {
    match d {
        saphyr::YamlData::Value(saphyr::Scalar::String(s)) => s.to_string(),
        saphyr::YamlData::Representation(s, _, _) => s.to_string(),
        _ => "?".to_string(),
    }
}

fn yaml_walk(node: &saphyr::MarkedYaml, prefix: &str, out: &mut Vec<(String, usize, usize)>) {
    match &node.data {
        saphyr::YamlData::Mapping(m) => {
            for (k, v) in m.iter() {
                let key = yaml_key(&k.data);
                let path = if prefix.is_empty() {
                    key
                } else {
                    format!("{prefix}.{key}")
                };
                out.push((path.clone(), v.span.start.line(), v.span.start.col()));
                yaml_walk(v, &path, out);
            }
        }
        saphyr::YamlData::Sequence(s) => {
            for (i, v) in s.iter().enumerate() {
                let path = format!("{prefix}[{i}]");
                out.push((path.clone(), v.span.start.line(), v.span.start.col()));
                yaml_walk(v, &path, out);
            }
        }
        _ => {}
    }
}

fn yaml_spans(src: &str) -> Vec<(String, usize, usize)> {
    use saphyr::LoadableYamlNode;
    let docs = saphyr::MarkedYaml::load_from_str(src).expect("yaml válido");
    let mut out = Vec::new();
    for doc in &docs {
        yaml_walk(doc, "", &mut out);
    }
    out
}

/// Baseline: parseo sin spans, sin recorrer el árbol.
fn yaml_plain(src: &str) -> usize {
    use saphyr::LoadableYamlNode;
    saphyr::Yaml::load_from_str(src).map_or(0, |d| d.len())
}

// --------------------------------------------------------------- TOML (toml_edit)

fn toml_walk(
    table: &toml_edit::Table,
    prefix: &str,
    idx: &LineIndex,
    out: &mut Vec<(String, usize, usize)>,
) {
    for (k, item) in table.iter() {
        let path = if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        };
        if let Some(r) = item.span() {
            let (l, c) = idx.line_col(r.start);
            out.push((path.clone(), l, c));
        }
        match item {
            toml_edit::Item::Table(t) => toml_walk(t, &path, idx, out),
            toml_edit::Item::Value(toml_edit::Value::InlineTable(t)) => {
                for (k2, v2) in t.iter() {
                    if let Some(r) = v2.span() {
                        let (l, c) = idx.line_col(r.start);
                        out.push((format!("{path}.{k2}"), l, c));
                    }
                }
            }
            _ => {}
        }
    }
}

fn toml_spans(src: &str) -> Vec<(String, usize, usize)> {
    let doc: toml_edit::Document<&str> = toml_edit::Document::parse(src).expect("toml válido");
    let idx = LineIndex::new(src);
    let mut out = Vec::new();
    toml_walk(doc.as_table(), "", &idx, &mut out);
    out
}

/// Baseline: parseo mutable, que descarta spans.
fn toml_plain(src: &str) -> usize {
    let doc: toml_edit::DocumentMut = src.parse().expect("toml válido");
    doc.as_table().len()
}

// ---------------------------------------------------------- JSON (jsonc-parser)

fn json_walk(
    v: &jsonc_parser::ast::Value,
    prefix: &str,
    idx: &LineIndex,
    out: &mut Vec<(String, usize, usize)>,
) {
    use jsonc_parser::ast::Value;
    match v {
        Value::Object(o) => {
            for prop in &o.properties {
                let key = prop.name.clone().into_string();
                let path = if prefix.is_empty() {
                    key
                } else {
                    format!("{prefix}.{key}")
                };
                let (l, c) = idx.line_col(prop.value.range().start);
                out.push((path.clone(), l, c));
                json_walk(&prop.value, &path, idx, out);
            }
        }
        Value::Array(a) => {
            for (i, item) in a.elements.iter().enumerate() {
                let path = format!("{prefix}[{i}]");
                let (l, c) = idx.line_col(item.range().start);
                out.push((path.clone(), l, c));
                json_walk(item, &path, idx, out);
            }
        }
        _ => {}
    }
}

fn json_spans(src: &str) -> Vec<(String, usize, usize)> {
    let res =
        jsonc_parser::parse_to_ast(src, &Default::default(), &Default::default()).expect("json válido");
    let idx = LineIndex::new(src);
    let mut out = Vec::new();
    if let Some(v) = res.value.as_ref() {
        json_walk(v, "", &idx, &mut out);
    }
    out
}

// ------------------------------------------------------------------- generadores

fn gen_yaml(n: usize) -> String {
    let mut s = String::from("db:\n  host: db.internal\n  port: 5432\n");
    for i in 0..n {
        s.push_str(&format!("k{i:04}: \"v{i}\"\n"));
    }
    s
}

fn gen_toml(n: usize) -> String {
    let mut s = String::from("[db]\nhost = \"db.internal\"\nport = 5432\n");
    for i in 0..n {
        s.push_str(&format!("k{i:04} = \"v{i}\"\n"));
    }
    s
}

fn gen_json(n: usize) -> String {
    let mut s = String::from("{\n  \"db\": {\n    \"host\": \"db.internal\",\n    \"port\": 5432\n  }");
    for i in 0..n {
        s.push_str(&format!(",\n  \"k{i:04}\": \"v{i}\""));
    }
    s.push_str("\n}\n");
    s
}

fn bench_us<F>(mut f: F, src: &str, iters: u32) -> f64
where
    F: FnMut(&str) -> usize,
{
    let _ = f(src);
    let start = Instant::now();
    let mut sink = 0usize;
    for _ in 0..iters {
        sink += f(src);
    }
    let elapsed = start.elapsed();
    std::hint::black_box(sink);
    elapsed.as_micros() as f64 / f64::from(iters)
}

fn main() {
    println!("== corrección de spans (fixture anidado) ==\n");

    let y = "db:\n  host: db.internal\n  port: 5432\n";
    let t = "[db]\nhost = \"db.internal\"\nport = 5432\n";
    let j = "{\n  \"db\": {\n    \"host\": \"db.internal\",\n    \"port\": 5432\n  }\n}\n";

    println!("YAML  {}", fmt(&yaml_spans(y)));
    println!("TOML  {}", fmt(&toml_spans(t)));
    println!("JSON  {}", fmt(&json_spans(j)));

    let n = 1000;
    let iters = 200;
    let (ys, ts, js) = (gen_yaml(n), gen_toml(n), gen_json(n));

    println!("\n== coste: 1000 claves, parseo + recorrido ({} iteraciones) ==\n", iters);
    println!("{:<22} {:>10} {:>10} {:>10}", "medición", "µs", "claves", "µs/clave");

    let rows = [
        ("YAML con spans", bench_us(|s| yaml_spans(s).len(), &ys, iters), n + 3),
        ("YAML sin spans", bench_us(yaml_plain, &ys, iters), n + 3),
        ("TOML con spans", bench_us(|s| toml_spans(s).len(), &ts, iters), n + 3),
        ("TOML sin spans", bench_us(toml_plain, &ts, iters), 1),
        ("JSON con spans", bench_us(|s| json_spans(s).len(), &js, iters), n + 3),
    ];
    for (name, us, keys) in rows {
        println!("{name:<22} {us:>10.1} {keys:>10} {:>10.2}", us / keys as f64);
    }

    let yaml_sp = rows[0].1;
    let yaml_plain_us = rows[1].1;
    println!(
        "\ncoste de la procedencia en YAML: +{:.0}% ({:.0} µs → {:.0} µs)",
        (yaml_sp / yaml_plain_us - 1.0) * 100.0,
        yaml_plain_us,
        yaml_sp
    );
    println!(
        "bytes: YAML {} · TOML {} · JSON {}",
        ys.len(),
        ts.len(),
        js.len()
    );
}

fn fmt(spans: &[(String, usize, usize)]) -> String {
    spans
        .iter()
        .map(|(k, l, c)| format!("{k}={l}:{c}"))
        .collect::<Vec<_>>()
        .join(" ")
}

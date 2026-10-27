# 0006 — Parsers con spans: saphyr, toml_edit y jsonc-parser

- Estado: aceptada
- Fecha: 2026-10-27
- Evidencia: [spike 0001](../spikes/0001-parsers-spans.md)

## Contexto

La regla dura 6 exige que toda clave expuesta en `explain()` conserve su span. El spike
0001 confirma que los tres formatos pueden darlo, con coste despreciable, pero solo con
las dependencias adecuadas.

## Decisión

| Formato | Crate | API |
|---|---|---|
| YAML | `saphyr` 0.3 | `MarkedYaml::load_from_str` → `span.start.line()/col()` |
| TOML | `toml_edit` 0.25 | `Document::parse` + `Item::span()` (bytes) |
| JSON | `jsonc-parser` 0.34 | `parse_to_ast` + `Ranged::range()` (bytes) |

Descartados:

- `serde_yaml` / `serde_yml`: archivados y sin spans.
- `marked-yaml`: duplica el parser (`yaml-rust2` 0.10) y `saphyr` ya lo incluye.
- `jiter`: rápido, pero sin posiciones.

## Consecuencias

- Tres crates de formato, cada uno tras su feature flag (`yaml`, `toml`, `json`).
- El core necesita un `LineIndex` por fichero para convertir offsets en línea/columna;
  sin él el parseo es cuadrático.
- Las columnas de TOML y JSON vienen en bytes y las de YAML en caracteres: el modelo de
  valores las normaliza a caracteres.
- Hace falta un test dorado con UTF-8 multibyte para fijar esa normalización.
- `deny.toml` mantiene el veto a `serde_yaml`/`serde_yml`, y el gate de CI mide con
  features por defecto para no confundirse con dependencias opcionales como la de
  `pyo3` en `jiter`.

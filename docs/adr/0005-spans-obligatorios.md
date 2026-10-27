# 0005 — Los parsers deben conservar spans

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

`explain()` promete decir en qué fichero y en qué línea se definió un valor. La
mayoría de los parsers de Rust desechan esa información al deserializar.

## Decisión

Los parsers del core deben producir valores con span (línea y columna). `serde_yaml`
está archivado y no los conserva, así que queda prohibido: YAML vía `saphyr`/
`marked_yaml`, TOML vía `toml_edit`, JSON con posiciones (`jiter` o índices propios).

## Consecuencias

- La elección de dependencias del parser la manda la procedencia, no la comodidad.
- `deny.toml` prohíbe `serde_yaml` y `serde_yml`.
- Plan B si un formato no expone spans: span `null` para ese formato, documentado,
  antes que renunciar a los demás.

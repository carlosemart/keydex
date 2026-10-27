# 0001 — Spans en los parsers

- Fecha: 2026-10-27
- Spike: [`spikes/spans/`](../../spikes/spans/)
- Pregunta: ¿podemos obtener línea y columna de cada clave en YAML, TOML y JSON, y a
  qué coste?

## Entorno

Rust 1.99.0, build `--release`, AMD Ryzen 9 7940HS. Fixture de 1000 claves planas más
3 anidadas; media de 200 iteraciones, incluyendo parseo **y** recorrido completo para
extraer las rutas.

## Corrección

Fixture anidado (`db.host` en línea 2 o 3 según formato). Columnas 0-based.

| Formato | Crate | `db.host` | Esperado |
|---|---|---|---|
| YAML | `saphyr` 0.3.3 | `2:8` | ✅ |
| TOML | `toml_edit` 0.25.17 | `2:7` | ✅ |
| JSON | `jsonc-parser` 0.34.0 | `3:12` | ✅ |

## Coste (1000 claves)

| Medición | µs | µs/clave |
|---|---|---|
| YAML con spans | 562 | 0.56 |
| YAML sin spans (solo parseo) | 500 | 0.50 |
| TOML con spans | 570 | 0.57 |
| TOML sin spans (solo parseo) | 574 | — |
| JSON con spans | 127 | 0.13 |

La procedencia cuesta **+12 % en YAML** y es indistinguible del ruido en TOML. Los tres
formatos quedan muy por debajo del presupuesto de 5 ms por 1000 claves.

## Hallazgos

1. **`saphyr` 0.3 ya trae `marked_yaml` dentro.** Expone `MarkedYaml { span, data }`,
   `Span { start, end }` y `Marker::line()` (1-based) / `col()` (0-based). No hace falta
   el crate `marked-yaml`, que además arrastra `yaml-rust2` 0.10 y duplicaría el parser.
2. **`toml_edit`: `DocumentMut` descarta los spans.** Hay que usar `Document`/`ImDocument`
   y `Item::span()`, que devuelve rangos en **bytes**.
3. **`jsonc-parser` es el más barato de los tres**: 1 crate en el cierre de dependencias,
   rangos en bytes y soporte de JSONC (comentarios y comas finales) gratis.
4. **`jiter` queda descartado**: no expone posiciones. Además su feature `python`
   arrastra `pyo3` (aparece en `Cargo.lock` aunque no se compile; el gate debe medir con
   features por defecto, nunca `--all-features`).
5. **Trampa medida.** Convertir byte → línea/columna recorriendo el fichero desde el
   inicio en cada clave hace el parseo cuadrático: 5,8 ms en TOML y 6,2 ms en JSON con
   1000 claves, frente a 0,57 y 0,13 con un índice de líneas y búsqueda binaria.
   **Regla para el core: un `LineIndex` por fichero, construido una vez.**
6. **Columnas heterogéneas.** `saphyr` da columna en caracteres; TOML y JSON en bytes.
   Con UTF-8 multibyte divergen: hay que normalizar a caracteres.

## Peso de dependencias (cierre completo)

| Crate | Crates en el cierre |
|---|---|
| `saphyr` | 20 |
| `toml_edit` | 9 |
| `jsonc-parser` | 1 |
| `marked-yaml` (descartado) | 15 |

## Límites de este spike

- Fixture casi todo ASCII y plano: falta un caso con acentos y emojis para fijar la
  normalización de columnas.
- No se midió YAML multi-documento (imports y perfiles) ni documentos de varios MB.
- Sin perfilado de memoria; solo tiempo.

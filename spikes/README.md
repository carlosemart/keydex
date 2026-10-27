# Spike: spans en los parsers

Crates de experimentación que **no forman parte del workspace** (cada uno declara su
propio `[workspace]`). No se publican y no se ejecutan en CI: son evidencia reproducible
de una decisión, no código de producción.

| Spike | Pregunta | Resultado |
|---|---|---|
| [`spans/`](spans/) | ¿YAML/TOML/JSON pueden darnos línea y columna, y a qué coste? | [docs/spikes/0001-parsers-spans.md](../../docs/spikes/0001-parsers-spans.md) |

## Ejecutar

```bash
cd spikes/spans
cargo run --release
```

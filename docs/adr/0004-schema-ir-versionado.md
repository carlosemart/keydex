# 0004 — El Schema IR se versiona aparte del paquete

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

El esquema viaja de Python a Rust en cada carga. Si evoluciona con el paquete, una
versión nueva del motor no puede leer un `dump` o un esquema antiguo, y los tests
cruzados dejan de ser reproducibles.

## Decisión

El IR lleva su propia versión en el campo `ir` (`keydex.schema/v1`). Dentro de `v1`
solo se admiten cambios aditivos. Cualquier ruptura crea `keydex.schema/v2` con soporte
simultáneo.

## Consecuencias

- Un `dump` de hace seis meses sigue siendo legible.
- Los fixtures compartidos entre Rust y Python se versionan con el IR, no con el crate.
- Publicar el core en crates.io no obliga a publicar el IR.

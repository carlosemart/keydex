# 0001 — Nombre: keydex

- Estado: aceptada
- Fecha: 2026-10-27

## Contexto

Las palabras inglesas del dominio (config, env, settings, merge, layer, canon,
provenance, whence, clew…) están ocupadas en PyPI y crates.io. `keydex` está libre
en ambos registros.

## Decisión

`keydex` como nombre del proyecto, del paquete de Python (`import keydex`) y del repo.
Crates: `keydex-core` (publicado) y `keydex-py` (no publicado).

## Consecuencias

- En GitHub existen repos homónimos no relacionados (uno de ellos, un gestor de
  contraseñas TUI con ~99 estrellas). Los identificadores que importan para instalar
  son PyPI y crates.io, y ahí el nombre es nuestro.
- `pip install keydex` y `cargo add keydex-core` son inequívocos.

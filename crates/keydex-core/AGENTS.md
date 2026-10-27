# keydex-core

Este crate se publica en crates.io: API pública mínima, `#[non_exhaustive]` en enums
públicos, semver 0.x, docs.rs con `all-features`.

- Cero `pyo3` y cero lectura directa de entorno, reloj o sistema de ficheros: todo
  entra inyectado.
- Los errores son enums (`thiserror`), nunca strings.
- Nada de `unwrap`/`expect`/`panic` fuera de `#[cfg(test)]`.
- Cualquier método de `std::env`, `std::time::SystemTime` o `std::fs` está bloqueado
  por `clippy.toml`; si necesitas uno, la solución es una abstracción inyectable.
- Todo lo público necesita documentación: `missing_docs` está en `deny`.

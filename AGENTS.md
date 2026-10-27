# keydex — instrucciones del proyecto

Especificación: `docs/spec.md`. Decisiones: `docs/adr/`. Léelos antes de tocar
`crates/`. Si el código y la spec discrepan, gana la spec: pregunta antes de cambiarla.

## Qué es keydex

Acceso a configuración: tipado, con procedencia y rápido. Pides una clave, recibes el
valor correcto, del origen correcto, con el tipo correcto, y puedes preguntar por qué.

## Qué NO es (no lo añadas aunque parezca útil)

Contenedor DI · motor de expresiones · framework de aplicación · FFI multi-lenguaje ·
hot reload por defecto · validación en Python.

## Reglas duras

1. `keydex-core` NO depende de `pyo3`.           → `just gate`
2. `keydex-core` NO lee entorno, reloj ni FS.    → `just gate` + clippy
3. Errores = enums `thiserror`. Nunca `String`.  → clippy `panic`/`unwrap_used`
4. Cero estado global mutable; caches con `RwLock`.
5. Ningún acceso a atributo cruza FFI; solo `build` y `refresh`.
6. Toda clave expuesta en `explain` conserva su span.

## Prohibiciones

- No `serde_yaml` ni `serde_yml` (archivados, sin spans): usa `saphyr`/`marked_yaml`.
- No `unwrap`/`expect`/`panic` fuera de `#[cfg(test)]`.
- No dependencias nuevas sin ADR y `cargo deny`.
- No cambiar `keydex.schema/v1` sin bump, fixture compartido y test cruzado.
- No `eval`, no red, no lectura de ficheros arbitrarios desde un resolver.

## Mapa

```
crates/keydex-core  motor puro: fuentes, precedencia, merge, interp, coerción
crates/keydex-py    binding PyO3 (F1), publish = false
python/keydex       API pública, tipos e integraciones (F1)
docs/spec.md        especificación vigente
docs/adr/           decisiones irreversibles
```

## Comandos

```
./scripts/check.sh     # fmt + clippy -D warnings
cargo test --workspace
./scripts/gate.sh      # invariantes de arquitectura (reglas 1 y 2)
```

## Terminado cuando

- [ ] hay un test que falla sin el cambio
- [ ] `just check`, `just gate` y `just test` en verde
- [ ] sin regresión de rendimiento cuando exista banco de pruebas
- [ ] `docs/spec.md` actualizado si cambió el comportamiento
- [ ] entrada en `CHANGELOG.md`

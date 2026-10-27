# keydex-core

Motor de configuración de [keydex](https://github.com/carlosemart/keydex):
fuentes, precedencia, merge, interpolación, coerción y procedencia.

Rust puro, sin dependencia de Python. Pensado para embeber en servicios Rust y
para servir de núcleo a la librería de Python.

## Invariantes

- No depende de `pyo3`.
- No lee entorno, reloj ni sistema de ficheros: todo entra inyectado.
- Errores estructurados (enums), nunca `String`.
- Cero estado global mutable.

## Estado

Pre-alpha. La API pública todavía no es estable.

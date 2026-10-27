# Benchmarks

Metodología: mediana de 50 ejecuciones, documento YAML de 1004 claves (10,7 KiB),
`env_prefix` sin variables, Python 3.15, AMD Ryzen 9 7940HS. Ejecutar con
`python/bench/bench_load.py`.

## F1 — carga (2026-10-27)

| Operación | Mediana |
|---|---|
| `keydex.load` | **1,26 ms** |
| `OmegaConf.load` | 26,42 ms |
| `cfg.explain("k0500")` | < 0,01 ms |
| acceso `oc.k0500` (OmegaConf) | < 0,01 ms |

**21× más rápido que OmegaConf** y dentro del presupuesto de la especificación
(< 5 ms para 1000 claves).

No es una comparación del todo justa, y conviene decirlo: `keydex.load` hace más trabajo
que OmegaConf —merge por bandas, expansión de `${...}`, `LineIndex` y procedencia por
valor—. El margen viene de no construir un árbol de objetos Python por nodo.

Pendiente de medir en fases siguientes: `refresh()`, decode a dataclass (F2) y accesos
masivos desde Python.

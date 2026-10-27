# Benchmarks

Methodology: median of 50 runs, YAML document with 1004 keys (10.7 KiB), `env_prefix`
without variables, Python 3.15, AMD Ryzen 9 7940HS. Run with
`python/bench/bench_load.py`.

## F1 — load (2026-10-27)

| Operation | Median |
|---|---|
| `keydex.load` | **1.26 ms** |
| `OmegaConf.load` | 26.42 ms |
| `cfg.explain("k0500")` | < 0.01 ms |
| `oc.k0500` access (OmegaConf) | < 0.01 ms |

**21× faster than OmegaConf**, inside the specification budget (< 5 ms for 1000 keys).

The comparison is not entirely fair, and it is worth saying: `keydex.load` does more work
than OmegaConf — band merging, `${...}` expansion, `LineIndex` and per-value provenance.
The margin comes from not building a tree of Python objects per node.

Still to measure in later phases: `refresh()`, dataclass decoding (F2) and bulk access
from Python.

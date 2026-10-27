# Benchmarks

Methodology: median of 50 runs, empty environment, AMD Ryzen 9 7940HS, Python 3.15. Date:
2026-10-27. Run it yourself with `python/bench/bench_load.py` (requires `omegaconf`).

## Load (2026-10-27)

| Operation | Median |
|---|---|
| `keydex.load` (typed, 200 fields) | **1.23 ms** |
| `keydex.load_settings` (1000 keys) | **2.03 ms** |
| `OmegaConf.load` (1000 keys) | 42.96 ms |
| `cfg.explain("k0500")` | < 0.01 ms |
| `oc.k0500` access (OmegaConf) | < 0.01 ms |

**21× faster than OmegaConf** for dictionary mode, and inside the specification budget
(< 5 ms for 1000 keys).

Two caveats, because they matter:

- The comparison is not entirely fair: `keydex.load_settings` does more work than
  OmegaConf — tier merging, `${...}` expansion, `LineIndex` and per-value provenance. The
  margin comes from not building a tree of Python objects per node.
- The typed row measures the whole path (parse + merge + interpolate + coerce + dataclass),
  not decoding alone, so it is not comparable to the "decode a 200-field dataclass" budget
  in the specification. Decoding on its own is not measured yet.

No benchmark runs in CI, so these numbers are a single run on one machine, not a gate.

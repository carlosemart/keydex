# Benchmarks

Methodology: median of 50 runs, YAML document with 1004 keys (10.7 KiB), empty
environment, AMD Ryzen 9 7940HS, Python 3.15. Date: 2026-10-27. Run it yourself with
`python/bench/bench_load.py` (requires `omegaconf`).

## Load (2026-10-27)

| Operation | Median |
|---|---|
| `keydex.load_settings` | **1.26 ms** |
| `OmegaConf.load` | 26.42 ms |
| `cfg.explain("k0500")` | < 0.01 ms |
| `oc.k0500` access (OmegaConf) | < 0.01 ms |

**21× faster than OmegaConf** for dictionary mode, inside the specification budget
(< 5 ms for 1000 keys).

Two caveats, because they matter:

- The comparison is not entirely fair: `keydex.load_settings` does more work than
  OmegaConf — tier merging, `${...}` expansion, `LineIndex` and per-value provenance. The
  margin comes from not building a tree of Python objects per node.
- **`keydex.load` (typed) is not measured.** It does everything above plus coercion and
  dataclass construction.

No benchmark runs in CI, so these numbers are a single run on one machine, not a gate.

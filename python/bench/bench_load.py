"""Load benchmark: keydex versus OmegaConf.

Usage:
    python/bench/bench_load.py [n_keys]

It is not a fair microbenchmark by design: keydex does more work (environment,
interpolation and per-value provenance). The goal is to show that the extra work is still
cheaper than building a tree of Python objects.
"""

from __future__ import annotations

import statistics
import sys
import tempfile
import time
from pathlib import Path
from typing import Callable

import keydex
from omegaconf import OmegaConf

N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
ITERS = 50


def make_doc(n: int) -> Path:
    lines = ["db:", "  host: db.internal", "  port: 5432", '  url: "${db.host}:${db.port}"']
    lines += [f"k{i:04}: {i}" for i in range(n)]
    path = Path(tempfile.mkdtemp()) / "app.yaml"
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return path


def bench(label: str, fn: Callable[[], object], iters: int = ITERS) -> float:
    fn()
    samples = []
    for _ in range(iters):
        start = time.perf_counter()
        fn()
        samples.append((time.perf_counter() - start) * 1_000)
    median = statistics.median(samples)
    print(f"{label:<28} {median:>9.2f} ms")
    return median


def main() -> None:
    path = make_doc(N)
    size = path.stat().st_size
    print(f"\n{N + 4} keys · {size / 1024:.1f} KiB · median of {ITERS} runs\n")

    t_keydex = bench(
        "keydex.load",
        lambda: keydex.load_settings([str(path)], env_prefix="BENCH_", env={}),
    )
    t_omega = bench("OmegaConf.load", lambda: OmegaConf.load(path))

    cfg = keydex.load_settings([str(path)], env_prefix="BENCH_", env={})
    oc = OmegaConf.load(path)
    bench("keydex explicit (explain)", lambda: cfg.explain("k0500"))
    bench("OmegaConf access", lambda: oc.k0500)

    print(f"\nload ratio: {t_omega / t_keydex:.1f}x faster than OmegaConf")


if __name__ == "__main__":
    main()

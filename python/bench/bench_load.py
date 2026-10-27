"""Benchmark de carga: keydex frente a OmegaConf.

Uso:
    python/bench/bench_load.py [n_claves]

No es un microbenchmark justo por diseño: keydex hace más trabajo (env, interpolación y
procedencia por valor). El objetivo es demostrar que ese trabajo extra sigue siendo más
barato que construir un árbol de objetos Python.
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
    print(f"\n{N + 4} claves · {size / 1024:.1f} KiB · mediana de {ITERS} ejecuciones\n")

    t_keydex = bench(
        "keydex.load",
        lambda: keydex.load([str(path)], env_prefix="BENCH_", env={}),
    )
    t_omega = bench("OmegaConf.load", lambda: OmegaConf.load(path))

    cfg = keydex.load([str(path)], env_prefix="BENCH_", env={})
    oc = OmegaConf.load(path)
    bench("keydex explícito (explain)", lambda: cfg.explain("k0500"))
    bench("OmegaConf acceso", lambda: oc.k0500)

    print(f"\nratio de carga: {t_omega / t_keydex:.1f}x más rápido que OmegaConf")


if __name__ == "__main__":
    main()

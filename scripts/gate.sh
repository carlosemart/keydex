#!/usr/bin/env bash
# Gates de arquitectura de keydex (AGENTS.md, reglas duras 1 y 2).
set -euo pipefail

fail=0

deps="$(cargo tree -p keydex-core -e normal --prefix none)"

# Regla 1: el core no puede depender de pyo3.
if printf '%s\n' "$deps" | grep -qx 'pyo3'; then
  echo "FALLO: keydex-core depende de pyo3 (regla dura 1)" >&2
  fail=1
fi

# Dependencias prohibidas (serde_yaml está archivado y no conserva spans).
for banned in serde_yaml serde_yml; do
  if printf '%s\n' "$deps" | grep -qx "$banned"; then
    echo "FALLO: $banned está prohibido; usa saphyr o marked_yaml" >&2
    fail=1
  fi
done

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check || fail=1
else
  echo "· cargo-deny no instalado: se omite en local (el CI lo ejecuta)"
fi

if [ "$fail" -eq 0 ]; then
  echo "gate: OK"
fi
exit "$fail"

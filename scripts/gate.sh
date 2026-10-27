#!/usr/bin/env bash
# keydex architecture gates (AGENTS.md, hard rules 1 and 2).
set -euo pipefail

fail=0

deps="$(cargo tree -p keydex-core -e normal --prefix none)"

# Hard rule 1: the core must not depend on pyo3.
if printf '%s\n' "$deps" | grep -qx 'pyo3'; then
  echo "FAIL: keydex-core depends on pyo3 (hard rule 1)" >&2
  fail=1
fi

# Banned dependencies (serde_yaml is archived and keeps no spans).
for banned in serde_yaml serde_yml; do
  if printf '%s\n' "$deps" | grep -qx "$banned"; then
    echo "FAIL: $banned is banned; use saphyr or marked_yaml" >&2
    fail=1
  fi
done

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check || fail=1
else
  echo "· cargo-deny not installed: skipped locally (CI runs it)"
fi

if [ "$fail" -eq 0 ]; then
  echo "gate: OK"
fi
exit "$fail"

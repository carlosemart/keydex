#!/usr/bin/env bash
# keydex architecture gates (AGENTS.md, hard rules 1 and 2).
#
#   ./scripts/gate.sh             run the gates
#   ./scripts/gate.sh --self-test prove that the gates fail on a violation
set -euo pipefail

# `cargo tree --prefix none` prints "pyo3 v0.30.1", not "pyo3", so the pattern has to
# match the name followed by a space or end of line.
has_crate() {
  local deps="$1" name="$2"
  printf '%s\n' "$deps" | grep -qE "^${name}( |\$)"
}

BANNED=(serde_yaml serde_yml)

check_deps() {
  local deps="$1" fail=0 banned

  if has_crate "$deps" pyo3; then
    echo "FAIL: keydex-core depends on pyo3 (hard rule 1)" >&2
    fail=1
  fi

  for banned in "${BANNED[@]}"; do
    if has_crate "$deps" "$banned"; then
      echo "FAIL: $banned is banned; use saphyr or marked_yaml" >&2
      fail=1
    fi
  done

  return "$fail"
}

self_test() {
  local fail=0

  if check_deps $'saphyr v0.3.3\npyo3 v0.30.1' >/dev/null 2>&1; then
    echo "FAIL: the pyo3 gate does not fire on a violation" >&2
    fail=1
  fi

  if check_deps $'saphyr v0.3.3\nserde_yaml v0.9.0' >/dev/null 2>&1; then
    echo "FAIL: the banned-crate gate does not fire on a violation" >&2
    fail=1
  fi

  if ! check_deps $'saphyr v0.3.3\ntoml_edit v0.25.0' >/dev/null 2>&1; then
    echo "FAIL: the gates reject a clean dependency list" >&2
    fail=1
  fi

  if [ "$fail" -eq 0 ]; then
    echo "gate self-test: OK"
  fi
  return "$fail"
}

if [ "${1:-}" = "--self-test" ]; then
  self_test
  exit $?
fi

fail=0
deps="$(cargo tree -p keydex-core -e normal --prefix none)"
check_deps "$deps" || fail=1

if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check || fail=1
else
  echo "· cargo-deny not installed: skipped locally (CI runs it)"
fi

if [ "$fail" -eq 0 ]; then
  echo "gate: OK"
fi
exit "$fail"

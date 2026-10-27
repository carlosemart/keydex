#!/usr/bin/env bash
# Build the documentation for both ecosystems.
#
#   ./scripts/docs.sh          build everything (site/ for Python, target/doc for Rust)
#   ./scripts/docs.sh serve    live preview of the Python site on http://127.0.0.1:8000
#   ./scripts/docs.sh rust     build and open the Rust API docs
#
# The Python site needs mkdocs-material, mkdocstrings[python] and ruff. They are fetched
# on demand with `uvx`, so nothing is installed globally.
set -euo pipefail

MKDOCS=(uvx --with mkdocs-material --with "mkdocstrings[python]" --with ruff mkdocs)

build_rust() {
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features -p keydex-core
}

case "${1:-build}" in
  build)
    build_rust
    "${MKDOCS[@]}" build --strict
    printf '\nPython site: file://%s/site/index.html\n' "$PWD"
    printf 'Rust API:    file://%s/target/doc/keydex_core/index.html\n' "$PWD"
    ;;
  serve)
    printf 'Serving on http://127.0.0.1:8000 (Ctrl-C to stop)\n'
    "${MKDOCS[@]}" serve
    ;;
  rust)
    build_rust
    cargo doc --no-deps --all-features -p keydex-core --open
    ;;
  *)
    echo "usage: $0 [build|serve|rust]" >&2
    exit 2
    ;;
esac

#!/usr/bin/env bash
# Comprobaciones previas a commit: formato y lints.
#
# `keydex-py` se excluye del clippy de workspace porque enlaza con `extension-module`
# (necesita libpython); se valida aparte con `cargo check`.
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --exclude keydex-py -- -D warnings
cargo check -p keydex-py

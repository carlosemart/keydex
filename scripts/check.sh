#!/usr/bin/env bash
# Pre-commit checks: formatting and lints.
#
# `keydex-py` is excluded from the workspace clippy run because it links with
# `extension-module` (it needs libpython); it is validated separately with `cargo check`.
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --exclude keydex-py -- -D warnings
cargo check -p keydex-py

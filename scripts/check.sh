#!/usr/bin/env bash
# Comprobaciones previas a commit: formato y lints.
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

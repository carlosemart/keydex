# keydex — development tasks
# The scripts are the source of truth; `just` is only a shortcut.
set shell := ["bash", "-euo", "pipefail", "-c"]

# Everything that must pass before committing
check:
    ./scripts/check.sh

# Workspace tests
test:
    cargo test --workspace --exclude keydex-py

# Architecture invariants (hard rules 1 and 2 of AGENTS.md)
gate:
    ./scripts/gate.sh

# Apply formatting
fmt:
    cargo fmt --all

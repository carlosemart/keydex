# keydex — tareas de desarrollo
# Los scripts son la fuente de verdad; `just` es solo un atajo.
set shell := ["bash", "-euo", "pipefail", "-c"]

# Todo lo que debe pasar antes de commitear
check:
    ./scripts/check.sh

# Tests del workspace
test:
    cargo test --workspace

# Invariantes de arquitectura (reglas 1 y 2 de AGENTS.md)
gate:
    ./scripts/gate.sh

# Aplicar formato
fmt:
    cargo fmt --all

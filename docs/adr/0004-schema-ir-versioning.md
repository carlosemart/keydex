# 0004 — The Schema IR is versioned apart from the package

- Status: accepted
- Date: 2026-10-27

## Context

The schema travels from Python to Rust on every load. If it evolved with the package, a new
engine version could not read an old dump or schema, and cross tests would stop being
reproducible.

## Decision

The IR carries its own version in the `ir` field (`keydex.schema/v1`). Inside `v1` only
additive changes are allowed. Any break creates `keydex.schema/v2` with simultaneous
support.

## Consequences

- A dump from six months ago is still readable.
- Fixtures shared between Rust and Python are versioned with the IR, not with the crate.
- Publishing the core to crates.io does not force publishing the IR.

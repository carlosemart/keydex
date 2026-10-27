# 0008 — Lock-free reads with `arc-swap`

- Status: accepted
- Date: 2026-10-27

## Context

`Settings` held the snapshot behind `RwLock<Arc<Snapshot>>`. Every `get`, `explain` or
`items` call took a read lock and cloned the `Arc`, which contradicts the claim that the
lock is only touched on `refresh()`. It also makes the free-threaded wheel pointless: a
`gil_used = false` module that serialises reads on a lock is not free-threaded in any
useful sense.

## Decision

`arc-swap` in `keydex-py` only. Readers do `load_full()` (an atomic increment, no lock);
`refresh()` builds the new snapshot and swaps the pointer.

## Consequences

- One new dependency, only in the binding. The core is unchanged.
- Reads are lock-free and never block a refresh; a reader keeps a coherent snapshot for
  the duration of the call.
- `Snapshot` must stay immutable, which it already is.
- The free-threaded wheel becomes meaningful, so it can be published once the benchmark
  proves the read path is not the bottleneck.

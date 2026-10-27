# 0002 — Interpolation does not support expressions

- Status: accepted
- Date: 2026-10-27

## Context

OmegaConf allows expressions in resolvers; Spring only allows references and placeholders.
An expression language implies a custom parser, type semantics, edge cases and an attack
surface.

## Decision

Interpolation supports only references (`${a.b}`, `${.sibling}`, `${..parent}`), a default
value (`${a:-x}`) and registered pure resolvers (`${env:VAR}`). No arithmetic, no
conditionals, no method access, no `eval`.

## Consequences

- The engine is portable to Rust without an interpreter.
- A derived value is computed in user code, not in the configuration.
- If it is ever needed, it ships as a plugin and never in the core.

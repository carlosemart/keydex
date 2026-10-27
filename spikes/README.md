# Spikes: parser spans

Experimental crates that are **not part of the workspace** (each declares its own
`[workspace]`). They are not published and do not run in CI: they are reproducible
evidence for a decision, not production code.

| Spike | Question | Result |
|---|---|---|
| [`spans/`](spans/) | Can YAML/TOML/JSON give us line and column, and at what cost? | Summarised in [the architecture](../../docs/architecture.md#why-these-choices) |

## Run

```bash
cd spikes/spans
cargo run --release
```

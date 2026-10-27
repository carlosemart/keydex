# Install

## Requirements

| Tool | Version | Why |
|---|---|---|
| Python | 3.11 or newer | `tomllib` in the standard library, `X \| Y` annotations |
| Rust | 1.85 or newer (`rust-toolchain.toml` pins stable) | builds the extension |
| [uv](https://docs.astral.sh/uv/) | any recent | creates the venv and installs |

`maturin` is fetched automatically by the build backend.

## From the repository

```bash
git clone https://github.com/carlosemart/keydex
cd keydex
uv venv
uv pip install -e .
```

`uv pip install -e .` builds the Rust extension and installs `keydex` in editable mode, so
Python changes apply immediately. Rust changes need the command again (or
`maturin develop`).

Verify:

```bash
.venv/bin/python -c "import keydex; print(keydex.__version__)"
```

## From PyPI

Not available yet: PyPI only holds the `0.0.1` name reservation, which installs an empty
placeholder. Do not `pip install keydex` expecting this code until the status page says
`0.1.0` is published.

## Wheels

| Wheel | Covers |
|---|---|
| `cp311-abi3` | Python 3.11–3.15 with the GIL |
| `cp314t` | 3.14 free-threaded, built and tested but **not published** yet |

There is no pure-Python fallback: the engine is Rust.

## Rust only

```toml
[dependencies]
keydex-core = { git = "https://github.com/carlosemart/keydex", branch = "main" }
```

Once `0.1.0` is on crates.io this becomes `cargo add keydex-core`.

## Troubleshooting the install

- **`error: can't find Rust compiler`** — install Rust (`rustup`) and reopen the shell.
- **`maturin` build fails on macOS** — make sure the Xcode command line tools are present.
- **`ModuleNotFoundError: keydex._keydex`** — the extension was not built; rerun
  `uv pip install -e .` and check that no `_keydex*.so` from another Python version is left
  in `python/keydex/`.

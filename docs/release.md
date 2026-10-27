# Release

## Before the first release

Two things only the account owner can do:

1. **PyPI — Trusted Publishing.** At
   <https://pypi.org/manage/project/keydex/settings/publishing/> add a publisher: owner
   `carlosemart`, repo `keydex`, workflow `release.yml`, environment `pypi`. The workflow
   then gets ephemeral credentials through OIDC and no token is stored.
2. **crates.io — publish token.** Create a token *scoped* to `keydex-core` and store it as
   the `CARGO_REGISTRY_TOKEN` repository secret. The workflow passes it through the
   environment, never through `argv`.

## Release

```bash
# 1. Bump the version in Cargo.toml (workspace) and add a CHANGELOG.md entry
# 2. Open a pull request with the six CI checks green and merge it
# 3. Tag the merge commit and push the tag
git tag -a v0.1.0 -m "0.1.0"
git push origin v0.1.0
```

The tag triggers `.github/workflows/release.yml`:

| Job | Output |
|---|---|
| `sdist` | source tarball |
| `wheels` | `cp311-abi3` for x86_64 and aarch64, manylinux 2.28 |
| `publish-pypi` | uploads the sdist and the wheels to PyPI |
| `publish-crates` | publishes `keydex-core` |

The free-threaded wheel (`cp314t`) is built and tested in CI but **not published yet**:
every access takes an `RwLock`, which defeats the point of free-threading. It ships once
the read path is lock-free.

## Why two builds

`abi3` and free-threading are incompatible today: an `abi3` wheel does not load in a `t`
interpreter, and `abi3t` (PEP 803) is not available yet. That is why the crate has the
`abi3` feature and the free-threaded build uses `--no-default-features`.

## Verification

```bash
# normal wheel in a clean venv
uv venv /tmp/v --python 3.14
uv pip install --python /tmp/v/bin/python dist/*.whl pytest
/tmp/v/bin/python -m pytest python/tests examples/homelab-demo/tests -q

# real free-threading
uv python install 3.14t
uv venv /tmp/vft --python 3.14t
maturin build --release --no-default-features --interpreter /tmp/vft/bin/python --out dist-ft
uv pip install --python /tmp/vft/bin/python dist-ft/*.whl pytest
/tmp/vft/bin/python -c "import sys, keydex; assert not sys._is_gil_enabled()"
/tmp/vft/bin/python -m pytest python/tests examples/homelab-demo/tests -q
```

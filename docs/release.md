# Publicación

## Antes de la primera release

Dos cosas que solo puede hacer el titular de las cuentas:

1. **PyPI — Trusted Publishing.** En <https://pypi.org/manage/project/keydex/settings/publishing/>
   añadir un publisher: owner `carlosemart`, repo `keydex`, workflow `release.yml`,
   environment `pypi`. Con eso el workflow obtiene credenciales efímeras por OIDC y no
   hay ningún token guardado.
2. **crates.io — token de publicación.** Crear un token *scoped* a `keydex-core` y
   guardarlo como secret `CARGO_REGISTRY_TOKEN` del repo. El workflow lo pasa por
   entorno, nunca por `argv`.

## Release

```bash
# 1. Bump de versión en Cargo.toml (workspace) y entrada en CHANGELOG.md
# 2. Commit y push a main
git tag -a v0.1.0 -m "0.1.0"
git push origin v0.1.0
```

El tag dispara `.github/workflows/release.yml`:

| Job | Salida |
|---|---|
| `sdist` | tarball de fuentes |
| `wheels` | `cp311-abi3` para x86_64 y aarch64, manylinux 2.28 |
| `wheels-freethreaded` | `cp314t` (sin `abi3`, con `gil_used = false`) |
| `publish-pypi` | sube todo a PyPI |
| `publish-crates` | publica `keydex-core` |

## Por qué dos builds

`abi3` y free-threading son incompatibles hoy: un wheel `abi3` no carga en un intérprete
`t`, y el `abi3t` (PEP 803) todavía no está disponible. Por eso el crate tiene la feature
`abi3` y el build free-threaded se hace con `--no-default-features`.

## Verificación

```bash
# wheel normal en un venv limpio
uv venv /tmp/v --python 3.14
uv pip install --python /tmp/v/bin/python dist/*.whl pytest
/tmp/v/bin/python -m pytest python/tests examples/homelab-demo/tests -q

# free-threading real
uv python install 3.14t
uv venv /tmp/vft --python 3.14t
maturin build --release --no-default-features --interpreter /tmp/vft/bin/python --out dist-ft
uv pip install --python /tmp/vft/bin/python dist-ft/*.whl pytest
/tmp/vft/bin/python -c "import sys, keydex; assert not sys._is_gil_enabled()"
/tmp/vft/bin/python -m pytest python/tests examples/homelab-demo/tests -q
```

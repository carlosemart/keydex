"""keydex — acceso tipado y explicable a configuración.

El motor vive en Rust; esta capa expone la API pública, construye el Schema IR a partir
de los dataclasses y materializa el resultado.
"""

from __future__ import annotations

import json
from typing import TypeVar

from keydex import _schema
from keydex._keydex import (
    KeydexError,
    Settings,
    load as _load_settings,
    load_typed as _load_typed,
)

__all__ = [
    "KeydexError",
    "Settings",
    "ValidationError",
    "load",
    "load_settings",
    "__version__",
]
__version__ = "0.0.1"

T = TypeVar("T")


class ValidationError(KeydexError):
    """El snapshot no cumple el esquema del modelo.

    `errors` es una lista de diccionarios con `path` y `message`, una por problema.
    """

    def __init__(self, errors: list[dict[str, str]]) -> None:
        self.errors = errors
        summary = "; ".join(f"{error['path']}: {error['message']}" for error in errors)
        super().__init__(summary or "validación fallida")


def load(
    model: type[T],
    *,
    files: tuple[str, ...] | list[str] = (),
    env_prefix: str = "",
    env: dict[str, str] | None = None,
) -> T:
    """Carga la configuración y devuelve una instancia tipada del modelo."""
    ir = json.dumps(_schema.build_ir(model), separators=(",", ":"))
    ok, payload = _load_typed(ir, list(files), env_prefix, env)
    if not ok:
        raise ValidationError(payload)
    return _schema.build(model, payload)  # type: ignore[return-value]


def load_settings(
    files: tuple[str, ...] | list[str] = (),
    env_prefix: str = "",
    env: dict[str, str] | None = None,
) -> Settings:
    """Carga la configuración en modo diccionario, sin esquema."""
    return _load_settings(list(files), env_prefix, env)

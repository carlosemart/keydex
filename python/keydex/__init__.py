"""keydex — acceso tipado y explicable a configuración.

El motor vive en Rust; esta capa expone la API pública, construye el Schema IR a partir
de los dataclasses y materializa el resultado.
"""

from __future__ import annotations

import json
import os
import warnings as _warnings
from typing import TypeVar

from keydex import _schema
from keydex._keydex import (
    KeydexError,
    Settings,
    load as _load_settings,
    load_typed as _load_typed,
)
from keydex._secrets import Secret

__all__ = [
    "KeydexError",
    "Secret",
    "Settings",
    "UnknownKeyWarning",
    "ValidationError",
    "load",
    "load_settings",
    "__version__",
]
__version__ = "0.0.1"

T = TypeVar("T")


class UnknownKeyWarning(UserWarning):
    """Una clave de la configuración no la usa ninguna ruta del esquema.

    Casi siempre es un error de escritura o un nivel mal anidado: el valor se ignora en
    silencio y el override no aplica nunca.
    """


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
    profiles: tuple[str, ...] | list[str] = (),
    on_unknown: str = "warn",
) -> T:
    """Carga la configuración y devuelve una instancia tipada del modelo.

    `on_unknown` decide qué hacer con las claves que el esquema no usa: `"warn"`
    (por defecto, `UnknownKeyWarning`), `"error"` (`ValidationError`) o `"ignore"`.
    """
    ir = json.dumps(_schema.build_ir(model), separators=(",", ":"))
    errors, unknown, tree = _load_typed(
        ir,
        list(files),
        env_prefix,
        _environment(env, env_prefix),
        list(profiles),
        on_unknown,
    )
    if errors:
        raise ValidationError(errors)
    for item in unknown:
        _warnings.warn(
            f"{item['path']}: {item['message']}",
            UnknownKeyWarning,
            stacklevel=2,
        )
    return _schema.build(model, tree)  # type: ignore[return-value]


def load_settings(
    files: tuple[str, ...] | list[str] = (),
    env_prefix: str = "",
    env: dict[str, str] | None = None,
    profiles: tuple[str, ...] | list[str] = (),
) -> Settings:
    """Carga la configuración en modo diccionario, sin esquema."""
    return _load_settings(
        list(files), env_prefix, _environment(env, env_prefix), list(profiles)
    )


def _environment(env: dict[str, str] | None, prefix: str) -> dict[str, str]:
    """Materializa el entorno y resuelve la indirección `*_FILE`.

    `APP_DB_PASSWORD_FILE=/run/secrets/db` define `APP_DB_PASSWORD` con el contenido del
    fichero. La lectura ocurre aquí, no en el motor: el core no toca el sistema de
    ficheros.
    """
    source = dict(os.environ) if env is None else dict(env)

    for key in list(source):
        if not key.endswith("_FILE") or not key.startswith(prefix):
            continue
        reference = source[key]
        try:
            with open(reference, encoding="utf-8") as handle:
                source[key[: -len("_FILE")]] = handle.read().strip()
        except OSError as error:
            raise KeydexError(
                f"{key}: no se puede leer {reference}: {error}"
            ) from error

    return source

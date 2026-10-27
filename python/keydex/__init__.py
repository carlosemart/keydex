"""keydex — typed, explainable configuration access.

The engine lives in Rust; this layer exposes the public API, builds the Schema IR from
dataclasses and materialises the result.
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
    __version__,
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

T = TypeVar("T")


class UnknownKeyWarning(UserWarning):
    """A configuration key is not used by any schema path.

    It is almost always a typo or a wrongly nested level: the value is silently ignored
    and the override never applies.
    """


class ValidationError(KeydexError):
    """The snapshot does not satisfy the model schema.

    `errors` is a list of dictionaries with `path` and `message`, one per problem.
    """

    def __init__(self, errors: list[dict[str, str]]) -> None:
        self.errors = errors
        summary = "; ".join(f"{error['path']}: {error['message']}" for error in errors)
        super().__init__(summary or "validation failed")


def load(
    model: type[T],
    *,
    files: tuple[str, ...] | list[str] = (),
    env_prefix: str = "",
    env: dict[str, str] | None = None,
    profiles: tuple[str, ...] | list[str] = (),
    on_unknown: str = "warn",
) -> T:
    """Loads the configuration and returns a typed instance of the model.

    `on_unknown` decides what happens with keys the schema does not use: `"warn"`
    (default, `UnknownKeyWarning`), `"error"` (`ValidationError`) or `"ignore"`.
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
    """Loads the configuration in dictionary mode, without a schema."""
    return _load_settings(
        list(files), env_prefix, _environment(env, env_prefix), list(profiles)
    )


def _environment(env: dict[str, str] | None, prefix: str) -> dict[str, str]:
    """Materialises the environment and resolves the `*_FILE` indirection.

    `APP_DB_PASSWORD_FILE=/run/secrets/db` defines `APP_DB_PASSWORD` with the file
    contents. Reading happens here, not in the engine: the core does not touch the
    filesystem.
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
                f"{key}: cannot read {reference}: {error}"
            ) from error

    return source

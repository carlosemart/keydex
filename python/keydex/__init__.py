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

    Attributes:
        errors: One dictionary per problem, with `path` and `message` keys.
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

    Args:
        model: Dataclass describing the configuration. Field types drive coercion and
            fields without a default are required.
        files: Configuration files, in application order.
        env_prefix: Prefix stripped from environment variables, for example `"APP_"`.
        env: Environment to use. Defaults to `os.environ`; tests should inject one
            instead of mutating the process environment.
        profiles: Active profiles. `app.yaml` plus `"prod"` applies `app-prod.yaml`.
        on_unknown: What to do with keys no field uses: `"warn"` (default,
            :class:`UnknownKeyWarning`), `"error"` (:class:`ValidationError`) or
            `"ignore"`.

    Returns:
        An instance of ``model``.

    Raises:
        ValidationError: The configuration does not satisfy the model schema.
        KeydexError: A file cannot be read or the engine fails.
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
    """Loads the configuration in dictionary mode, without a schema.

    Args:
        files: Configuration files, in application order.
        env_prefix: Prefix stripped from environment variables.
        env: Environment to use. Defaults to `os.environ`.
        profiles: Active profiles, applied as ``<name>-<profile>.<ext>``.

    Returns:
        A :class:`Settings` mapping with provenance support.
    """
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

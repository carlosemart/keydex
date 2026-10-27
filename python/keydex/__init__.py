"""keydex — typed, explainable configuration access.

The engine lives in Rust; this layer exposes the public API, builds the Schema IR from
dataclasses and materialises the result.
"""

from __future__ import annotations

import json
import os
import warnings as _warnings
from pathlib import Path
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
    secret_files: bool = False,
    secret_root: str | os.PathLike[str] | None = None,
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
        secret_files: Resolve `*_FILE` variables by reading the referenced file. Off by
            default: it turns the environment into a file-read surface.
        secret_root: Directory that resolved `*_FILE` paths must stay inside. Required
            when `secret_files` is enabled.

    Returns:
        An instance of ``model``.

    Raises:
        ValidationError: The configuration does not satisfy the model schema.
        KeydexError: A file cannot be read, a `*_FILE` path escapes `secret_root`, or the
            engine fails.
    """
    environment, _ = _environment(
        env, env_prefix, secret_files=secret_files, secret_root=secret_root
    )
    ir = json.dumps(_schema.build_ir(model), separators=(",", ":"))
    errors, unknown, tree = _load_typed(
        ir,
        list(files),
        env_prefix,
        environment,
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
    *,
    sensitive: tuple[str, ...] | list[str] = (),
    secret_files: bool = False,
    secret_root: str | os.PathLike[str] | None = None,
) -> Settings:
    """Loads the configuration in dictionary mode, without a schema.

    Args:
        files: Configuration files, in application order.
        env_prefix: Prefix stripped from environment variables.
        env: Environment to use. Defaults to `os.environ`.
        profiles: Active profiles, applied as ``<name>-<profile>.<ext>``.
        sensitive: Keys hidden from `explain`, `items` and `dump`. Values loaded from a
            `*_FILE` variable are added automatically.
        secret_files: Resolve `*_FILE` variables by reading the referenced file.
        secret_root: Directory that resolved `*_FILE` paths must stay inside.

    Returns:
        A :class:`Settings` mapping with provenance support.
    """
    environment, derived = _environment(
        env, env_prefix, secret_files=secret_files, secret_root=secret_root
    )
    return _load_settings(
        list(files),
        env_prefix,
        environment,
        list(profiles),
        list(sensitive) + derived,
    )


def _environment(
    env: dict[str, str] | None,
    prefix: str,
    *,
    secret_files: bool = False,
    secret_root: str | os.PathLike[str] | None = None,
) -> tuple[dict[str, str], list[str]]:
    """Materialises the environment and, when enabled, resolves `*_FILE` variables.

    `APP_DB_PASSWORD_FILE=/run/secrets/db` defines `APP_DB_PASSWORD` with the file
    contents and marks `db.password` as sensitive. Reading happens here, not in the
    engine: the core does not touch the filesystem.

    Returns:
        The environment plus the keys derived from files, which callers must treat as
        sensitive.
    """
    source = dict(os.environ) if env is None else dict(env)

    if not secret_files:
        return source, []

    if secret_root is None:
        raise KeydexError(
            "secret_root is required when secret_files is enabled; "
            "point it at the directory that holds the secret files"
        )

    root = Path(secret_root).expanduser().resolve()
    sensitive: list[str] = []

    for key in list(source):
        if not key.endswith("_FILE"):
            continue
        if prefix and not key.startswith(prefix):
            continue

        reference = Path(source[key]).expanduser()
        try:
            resolved = reference.resolve(strict=True)
        except OSError as error:
            raise KeydexError(f"{key}: cannot read {reference}: {error}") from error

        if not resolved.is_relative_to(root):
            raise KeydexError(
                f"{key}: {resolved} is outside secret_root ({root})"
            )

        try:
            source[key[: -len("_FILE")]] = resolved.read_text(encoding="utf-8").strip()
        except OSError as error:
            raise KeydexError(f"{key}: cannot read {resolved}: {error}") from error

        stem = key[: -len("_FILE")]
        suffix = stem[len(prefix) :] if prefix else stem
        sensitive.append(suffix.lower().replace("__", "."))

    return source, sensitive

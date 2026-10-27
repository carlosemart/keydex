"""keydex — typed, explainable configuration access.

The engine lives in Rust; this layer exposes the public API, builds the Schema IR from
dataclasses and materialises the result.
"""

from __future__ import annotations

import json
import os
import warnings as _warnings
from collections.abc import Sequence
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
    "Environment",
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
        errors: One dictionary per problem, with `path`, `message` and, when the value
            came from a file, `file`, `line` and `col`.
    """

    def __init__(self, errors: list[dict[str, str]]) -> None:
        self.errors = errors
        summary = "; ".join(f"{error['path']}: {error['message']}" for error in errors)
        super().__init__(summary or "validation failed")


class Environment:
    """One build, two views.

    `load(model)` returns the typed model; the same snapshot is kept for `explain`,
    `settings()` and `refresh()`, so provenance describes exactly what the model was built
    from without loading the configuration twice.

    Args:
        files: Configuration files, in application order.
        env_prefix: Prefix stripped from environment variables.
        env: Environment to use. Defaults to `os.environ` only when `env_prefix` is not
            empty; with no prefix, pass the variables explicitly.
        profiles: Active profiles, applied as ``<name>-<profile>.<ext>``.
        on_unknown: `"warn"`, `"error"` or `"ignore"` for keys the schema does not use.
        sensitive: Extra keys hidden from `explain`, `items` and `dump`.
        secret_files: Resolve `*_FILE` variables by reading the referenced files.
        secret_root: Directory that resolved `*_FILE` paths must stay inside.
    """

    def __init__(
        self,
        *,
        files: Sequence[str] = (),
        env_prefix: str = "",
        env: dict[str, str] | None = None,
        profiles: Sequence[str] = (),
        on_unknown: str = "warn",
        sensitive: Sequence[str] = (),
        secret_files: bool = False,
        secret_root: str | os.PathLike[str] | None = None,
    ) -> None:
        self._files = list(files)
        self._env_prefix = env_prefix
        self._env = env
        self._profiles = list(profiles)
        self._on_unknown = on_unknown
        self._sensitive = list(sensitive)
        self._secret_files = secret_files
        self._secret_root = secret_root
        self._settings: Settings | None = None

    def load(self, model: type[T]) -> T:
        """Builds the configuration and returns the typed model.

        Raises:
            ValidationError: The configuration does not satisfy the model schema.
            KeydexError: A file cannot be read or the engine fails.
        """
        source, secret_vars = _environment(
            self._env,
            self._env_prefix,
            secret_files=self._secret_files,
            secret_root=self._secret_root,
        )
        ir = json.dumps(_schema.build_ir(model), separators=(",", ":"))
        settings, errors, unknown, tree = _load_typed(
            ir,
            self._files,
            self._env_prefix,
            source,
            self._profiles,
            self._on_unknown,
            list(self._sensitive) + _schema.sensitive_paths(model),
            secret_vars,
            self._env is None,
            self._secret_files,
            str(self._secret_root) if self._secret_root is not None else None,
        )
        if errors:
            raise ValidationError(errors)
        for item in unknown:
            _warnings.warn(
                f"{item['path']}: {item['message']}",
                UnknownKeyWarning,
                stacklevel=2,
            )
        self._settings = settings
        return _schema.build(model, tree)  # type: ignore[return-value]

    def explain(self, key: str, *, reveal: bool = False) -> dict[str, object] | None:
        """Provenance of a key in the snapshot used by the last `load`."""
        return self._require().explain(key, reveal=reveal)

    def settings(self) -> Settings:
        """The dictionary view of the same snapshot."""
        return self._require()

    def refresh(self) -> None:
        """Re-reads the sources and validates them.

        A typed model already returned by `load` is a copy: call `load(model)` again to get
        an updated one.
        """
        self._require().refresh()

    def _require(self) -> Settings:
        if self._settings is None:
            raise KeydexError("call load() before using the environment")
        return self._settings


def load(
    model: type[T],
    *,
    files: tuple[str, ...] | list[str] = (),
    env_prefix: str = "",
    env: dict[str, str] | None = None,
    profiles: tuple[str, ...] | list[str] = (),
    on_unknown: str = "warn",
    sensitive: tuple[str, ...] | list[str] = (),
    secret_files: bool = False,
    secret_root: str | os.PathLike[str] | None = None,
) -> T:
    """Loads the configuration and returns a typed instance of the model.

    A convenience wrapper over :class:`Environment`; use the class when you also need
    `explain` or `refresh`.

    Args:
        model: Dataclass describing the configuration.
        files: Configuration files, in application order.
        env_prefix: Prefix stripped from environment variables, for example `"APP_"`.
        env: Environment to use. Defaults to `os.environ` only when `env_prefix` is not
            empty; with no prefix, pass the variables explicitly.
        profiles: Active profiles.
        on_unknown: `"warn"`, `"error"` or `"ignore"`.
        sensitive: Extra keys hidden from `explain`, `items` and `dump`.
        secret_files: Resolve `*_FILE` variables by reading the referenced files.
        secret_root: Directory that resolved `*_FILE` paths must stay inside.

    Returns:
        An instance of ``model``.

    Raises:
        ValidationError: The configuration does not satisfy the model schema.
        KeydexError: A file cannot be read or the engine fails.
    """
    return Environment(
        files=files,
        env_prefix=env_prefix,
        env=env,
        profiles=profiles,
        on_unknown=on_unknown,
        sensitive=sensitive,
        secret_files=secret_files,
        secret_root=secret_root,
    ).load(model)


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
        env: Environment to use. Defaults to `os.environ` only when `env_prefix` is not
            empty; with no prefix, pass the variables explicitly.
        profiles: Active profiles.
        sensitive: Keys hidden from `explain`, `items` and `dump`.
        secret_files: Resolve `*_FILE` variables by reading the referenced files.
        secret_root: Directory that resolved `*_FILE` paths must stay inside.

    Returns:
        A :class:`Settings` mapping with provenance support.
    """
    source, secret_vars = _environment(
        env, env_prefix, secret_files=secret_files, secret_root=secret_root
    )
    return _load_settings(
        list(files),
        env_prefix,
        source,
        list(profiles),
        list(sensitive),
        secret_vars,
        env is None,
        secret_files,
        str(secret_root) if secret_root is not None else None,
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
        The environment plus the names of the variables that were resolved from a file.
        The engine marks whatever key those variables bind to, using the same binder that
        resolves them, so relaxed binding cannot desynchronise the mark.
    """
    # With no prefix and no injected environment, reading the process environment would
    # let USER, HOME or PATH silently override configuration. Require one of the two.
    source = (dict(os.environ) if prefix else {}) if env is None else dict(env)

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
            raise KeydexError(f"{key}: {resolved} is outside secret_root ({root})")

        try:
            source[key[: -len("_FILE")]] = resolved.read_text(encoding="utf-8").strip()
        except OSError as error:
            raise KeydexError(f"{key}: cannot read {resolved}: {error}") from error

        # The path itself is not configuration: keeping it would expose the location of
        # the secret and show up as an unknown key.
        del source[key]

        sensitive.append(key[: -len("_FILE")])

    return source, sensitive

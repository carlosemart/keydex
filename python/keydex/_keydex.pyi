"""Type stubs for the compiled extension module."""

from collections.abc import Sequence
from typing import Any

__version__: str

class KeydexError(Exception):
    """Base error raised by the keydex engine."""

class Settings:
    """Resolved configuration, immutable and queryable by key."""

    def get(self, key: str, default: Any = None) -> Any:
        """Returns the value of `key`, or `default` when it does not exist."""

    def __getitem__(self, key: str) -> Any:
        """Returns the value of `key`; sensitive keys yield `None`.

        Raises `KeyError` when the key does not exist. `get()` returns the raw value.
        """

    def __contains__(self, key: str) -> bool: ...
    def __len__(self) -> int: ...
    def keys(self) -> list[str]:
        """Keys in lexicographic order."""

    def items(self) -> list[tuple[str, Any]]:
        """`(key, value)` pairs; sensitive values are returned as `None`."""

    def explain(self, key: str, reveal: bool = False) -> dict[str, Any] | None:
        """Provenance of `key`; sensitive values are redacted unless `reveal` is set."""

    def refresh(self) -> None:
        """Re-reads the sources and replaces the snapshot atomically."""

def load(
    files: Sequence[str],
    env_prefix: str = "",
    env: dict[str, str] | None = None,
    profiles: Sequence[str] = (),
    sensitive: Sequence[str] = (),
    sensitive_env: Sequence[str] = (),
    refresh_env: bool = True,
    secret_files: bool = False,
    secret_root: str | None = None,
) -> Settings:
    """Loads files plus environment in dictionary mode."""

def load_typed(
    ir: str,
    files: Sequence[str],
    env_prefix: str = "",
    env: dict[str, str] | None = None,
    profiles: Sequence[str] = (),
    on_unknown: str = "warn",
    sensitive: Sequence[str] = (),
    sensitive_env: Sequence[str] = (),
    refresh_env: bool = True,
    secret_files: bool = False,
    secret_root: str | None = None,
) -> tuple[Settings, list[dict[str, Any]], list[dict[str, str]], Any]:
    """Loads with a schema: returns `(settings, errors, warnings, tree)`."""

"""Sensitive values."""

from __future__ import annotations

from typing import Generic, TypeVar

T = TypeVar("T")


class Secret(Generic[T]):
    """Sensitive value whose representation never reveals it.

    Used in typed models: `password: Secret[str]`. The real value only leaves through
    :meth:`reveal`, so a `repr`, a log line or a `print` of the dataclass cannot leak it.
    """

    __slots__ = ("_value",)

    def __init__(self, value: T) -> None:
        self._value = value

    def reveal(self) -> T:
        """Returns the value in the clear. Use it only where it is consumed."""
        return self._value

    def __repr__(self) -> str:
        return "Secret('***')"

    def __str__(self) -> str:
        return "Secret('***')"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Secret):
            return NotImplemented
        return bool(self._value == other._value)

    def __hash__(self) -> int:
        return hash(self._value)

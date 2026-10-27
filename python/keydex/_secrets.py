"""Valores sensibles."""

from __future__ import annotations

from typing import Generic, TypeVar

T = TypeVar("T")


class Secret(Generic[T]):
    """Valor sensible cuya representación nunca lo revela.

    Se usa en modelos tipados: `password: Secret[str]`. El valor real solo sale con
    :meth:`reveal`, así que un `repr`, un log o un `print` del dataclass no lo filtran.
    """

    __slots__ = ("_value",)

    def __init__(self, value: T) -> None:
        self._value = value

    def reveal(self) -> T:
        """Devuelve el valor en claro. Úsalo solo en el punto donde se consume."""
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

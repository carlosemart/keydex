"""Schema IR: introspección del modelo y materialización del resultado.

Python genera el IR (contrato `keydex.schema/v1`), Rust hace binding, coerción y
validación, y aquí se reconstruye el dataclass. La conversión de tipos ricos (`Enum`,
`Path`, `Decimal`, `UUID`) también ocurre en esta capa, no en el motor.
"""

from __future__ import annotations

import dataclasses
import types
import typing
from decimal import Decimal
from enum import Enum
from pathlib import Path
from uuid import UUID

from keydex._secrets import Secret

IR_VERSION = "keydex.schema/v1"

_CONVERTERS: dict[type, str] = {Path: "path", Decimal: "decimal", UUID: "uuid"}


def build_ir(model: type) -> dict:
    """Construye el IR de un modelo tipado."""
    if not dataclasses.is_dataclass(model):
        raise TypeError(f"se esperaba un dataclass, llegó {model!r}")
    return {"ir": IR_VERSION, "root": _node(model, required=True)}


def _node(tp: object, *, required: bool) -> dict:
    origin = typing.get_origin(tp)

    if origin in (typing.Union, types.UnionType):
        args = [arg for arg in typing.get_args(tp) if arg is not type(None)]
        if len(args) != 1:
            raise TypeError(f"unión no soportada: {tp!r}")
        return {
            "type": "optional",
            "required": False,
            "inner": _node(args[0], required=False),
        }

    if origin is Secret:
        args = typing.get_args(tp)
        inner = args[0] if args else str
        if inner is not str:
            raise TypeError("Secret[T] solo admite str por ahora")
        return {"type": "str", "required": required, "converter": "secret"}

    if origin in (list, tuple):
        args = typing.get_args(tp)
        item = args[0] if args else str
        return {"type": "list", "required": required, "items": _node(item, required=False)}

    if dataclasses.is_dataclass(tp):
        hints = typing.get_type_hints(tp)
        fields = {}
        for field in dataclasses.fields(tp):
            has_default = (
                field.default is not dataclasses.MISSING
                or field.default_factory is not dataclasses.MISSING
            )
            fields[field.name] = _node(hints[field.name], required=not has_default)
        return {"type": "struct", "required": required, "fields": fields}

    if isinstance(tp, type) and issubclass(tp, Enum):
        return {"type": "str", "required": required, "converter": "enum"}

    if tp in _CONVERTERS:
        return {"type": "str", "required": required, "converter": _CONVERTERS[tp]}

    simple = {str: "str", int: "int", float: "float", bool: "bool"}
    if tp in simple:
        return {"type": simple[tp], "required": required}

    raise TypeError(f"tipo no soportado en el esquema: {tp!r}")


def build(model: type, tree: dict) -> object:
    """Reconstruye el modelo a partir del árbol tipado que devuelve el motor."""
    hints = typing.get_type_hints(model)
    kwargs = {
        field.name: _convert(hints[field.name], tree[field.name])
        for field in dataclasses.fields(model)
        if field.name in tree
    }
    return model(**kwargs)


def _convert(tp: object, value: object) -> object:
    if value is None:
        return None

    origin = typing.get_origin(tp)

    if origin in (typing.Union, types.UnionType):
        inner = [arg for arg in typing.get_args(tp) if arg is not type(None)]
        if not inner:
            return value
        return _convert(inner[0], value)

    if origin in (list, tuple):
        args = typing.get_args(tp)
        item = args[0] if args else str
        return [_convert(item, element) for element in typing.cast("list[object]", value)]

    if origin is Secret:
        return Secret(value)

    if dataclasses.is_dataclass(tp) and isinstance(value, dict):
        return build(typing.cast("type", tp), typing.cast("dict", value))

    if isinstance(tp, type) and issubclass(tp, Enum):
        return tp(value)

    if tp in _CONVERTERS:
        return typing.cast("typing.Callable[[object], object]", tp)(value)

    return value

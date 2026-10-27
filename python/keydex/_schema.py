"""Schema IR: model introspection and result materialisation.

Python generates the IR (the `keydex.schema/v1` contract), Rust does binding, coercion
and validation, and this layer rebuilds the dataclass. Rich type conversion (`Enum`,
`Path`, `Decimal`, `UUID`) also happens here, not in the engine.
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
    """Builds the IR for a typed model."""
    if not dataclasses.is_dataclass(model):
        raise TypeError(f"expected a dataclass, got {model!r}")
    return {"ir": IR_VERSION, "root": _node(model, required=True)}


def _node(tp: object, *, required: bool) -> dict:
    origin = typing.get_origin(tp)

    if origin in (typing.Union, types.UnionType):
        args = [arg for arg in typing.get_args(tp) if arg is not type(None)]
        if len(args) != 1:
            raise TypeError(f"unsupported union: {tp!r}")
        return {
            "type": "optional",
            "required": False,
            "inner": _node(args[0], required=False),
        }

    if origin is Secret:
        args = typing.get_args(tp)
        inner = args[0] if args else str
        if inner is not str:
            raise TypeError("Secret[T] only supports str for now")
        return {"type": "str", "required": required}

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
        return {"type": "str", "required": required}

    if tp in _CONVERTERS:
        return {"type": "str", "required": required}

    simple = {str: "str", int: "int", float: "float", bool: "bool"}
    if tp in simple:
        return {"type": simple[tp], "required": required}

    raise TypeError(f"unsupported schema type: {tp!r}")


def build(model: type, tree: dict) -> object:
    """Rebuilds the model from the typed tree returned by the engine."""
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

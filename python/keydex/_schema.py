"""Schema IR: model introspection and result materialisation.

Python generates the IR (the `keydex.schema/v1` contract), Rust does binding, coercion and
validation, and this layer rebuilds the dataclass. Rich types are validated in the engine;
here they are only constructed.
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


def build_ir(model: type) -> dict:
    """Builds the IR for a typed model."""
    if not dataclasses.is_dataclass(model):
        raise TypeError(f"expected a dataclass, got {model!r}")
    return {"ir": IR_VERSION, "root": _node(model, required=True)}


def _node(tp: object, *, required: bool, has_default: bool = False) -> dict:
    origin = typing.get_origin(tp)

    if origin in (typing.Union, types.UnionType):
        args = [arg for arg in typing.get_args(tp) if arg is not type(None)]
        if len(args) != 1:
            raise TypeError(f"unsupported union: {tp!r}")
        return {
            "type": "optional",
            "required": False,
            "has_default": has_default,
            "inner": _node(args[0], required=False),
        }

    if origin is Secret:
        args = typing.get_args(tp)
        inner = args[0] if args else str
        return {
            "type": "secret",
            "required": required,
            "inner": _node(inner, required=False),
        }

    if origin in (list, tuple):
        args = typing.get_args(tp)
        item = args[0] if args else str
        return {"type": "list", "required": required, "items": _node(item, required=False)}

    if origin is dict:
        args = typing.get_args(tp)
        key_type, value_type = args if len(args) == 2 else (str, str)
        if key_type is not str:
            raise TypeError("map keys must be str")
        return {
            "type": "map",
            "required": required,
            "values": _node(value_type, required=False),
        }

    if dataclasses.is_dataclass(tp):
        hints = typing.get_type_hints(tp)
        fields = {}
        for field in dataclasses.fields(tp):
            default = (
                field.default is not dataclasses.MISSING
                or field.default_factory is not dataclasses.MISSING
            )
            fields[field.name] = _node(
                hints[field.name], required=not default, has_default=default
            )
        return {
            "type": "struct",
            "required": required,
            "has_default": has_default,
            "fields": fields,
        }

    if isinstance(tp, type) and issubclass(tp, Enum):
        return {
            "type": "enum",
            "required": required,
            "variants": [str(member.value) for member in tp],
        }

    if tp is Path:
        return {"type": "path", "required": required}
    if tp is Decimal:
        return {"type": "decimal", "required": required}
    if tp is UUID:
        return {"type": "uuid", "required": required}
    if tp is typing.Any:
        return {"type": "any", "required": required}

    simple = {str: "str", int: "int", float: "float", bool: "bool"}
    if tp in simple:
        return {"type": simple[tp], "required": required}

    raise TypeError(f"unsupported schema type: {tp!r}")


def sensitive_paths(model: type, prefix: str = "") -> list[str]:
    """Paths of the fields declared as `Secret[...]`, so the engine can redact them."""
    if not dataclasses.is_dataclass(model):
        return []
    hints = typing.get_type_hints(model)
    paths: list[str] = []
    for field in dataclasses.fields(model):
        paths.extend(_sensitive_node(hints[field.name], f"{prefix}{field.name}"))
    return paths


def _sensitive_node(tp: object, path: str) -> list[str]:
    origin = typing.get_origin(tp)

    if origin in (typing.Union, types.UnionType):
        inner = [arg for arg in typing.get_args(tp) if arg is not type(None)]
        return _sensitive_node(inner[0], path) if inner else []

    if origin is Secret:
        return [path]

    if origin in (list, tuple):
        args = typing.get_args(tp)
        item = args[0] if args else str
        return [path] if item is Secret or typing.get_origin(item) is Secret else []

    if dataclasses.is_dataclass(tp):
        return sensitive_paths(typing.cast("type", tp), f"{path}.")

    return []


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

    if origin is dict:
        args = typing.get_args(tp)
        value_type = args[1] if len(args) == 2 else str
        entries = typing.cast("dict", value).items()
        return {key: _convert(value_type, item) for key, item in entries}

    if origin is Secret:
        return Secret(value)

    if dataclasses.is_dataclass(tp) and isinstance(value, dict):
        return build(typing.cast("type", tp), typing.cast("dict", value))

    if isinstance(tp, type) and issubclass(tp, Enum):
        wanted = str(value).lower()
        for member in tp:
            if str(member.value).lower() == wanted:
                return member
        return tp(value)

    if tp is Path:
        return Path(typing.cast("str", value))
    if tp is Decimal:
        return Decimal(typing.cast("str", value))
    if tp is UUID:
        return UUID(typing.cast("str", value))

    return value

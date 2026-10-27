"""Typed model tests (F2): coercion, relaxed binding and aggregated errors."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path

import pytest

import keydex


@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432
    tls: bool = False


@dataclass(frozen=True)
class App:
    db: Db
    debug: bool = False
    note: str | None = None
    tags: list[str] = field(default_factory=list)


YAML = """\
db:
  host: file.internal
  port: 5432
  tls: true
tags:
  - alpha
  - beta
"""


def write(tmp_path: Path, text: str) -> str:
    path = tmp_path / "app.yaml"
    path.write_text(text, encoding="utf-8")
    return str(path)


def test_returns_the_typed_model(tmp_path: Path) -> None:
    cfg = keydex.load(App, files=[write(tmp_path, YAML)], env_prefix="APP_", env={})

    assert isinstance(cfg, App)
    assert isinstance(cfg.db, Db)
    assert cfg.db.host == "file.internal"
    assert cfg.db.port == 5432
    assert isinstance(cfg.db.port, int)
    assert cfg.db.tls is True
    assert cfg.tags == ["alpha", "beta"]


def test_relaxed_binding_without_double_underscore(tmp_path: Path) -> None:
    cfg = keydex.load(
        App,
        files=[write(tmp_path, YAML)],
        env_prefix="APP_",
        env={"APP_DB_HOST": "env.internal", "APP_DB_PORT": "6000"},
    )
    assert cfg.db.host == "env.internal"
    assert cfg.db.port == 6000


def test_dataclass_defaults_are_respected(tmp_path: Path) -> None:
    cfg = keydex.load(
        App,
        files=[write(tmp_path, "")],
        env_prefix="APP_",
        env={"APP_DB_HOST": "h"},
    )
    assert cfg.db.port == 5432
    assert cfg.db.tls is False
    assert cfg.debug is False
    assert cfg.note is None
    assert cfg.tags == []


def test_validation_error_with_paths(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(App, files=[path], env_prefix="APP_", env={"APP_DB_PORT": "many"})

    paths = [error["path"] for error in info.value.errors]
    assert "db.port" in paths


def test_missing_required_field(tmp_path: Path) -> None:
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(App, files=[write(tmp_path, "")], env_prefix="APP_", env={})

    paths = [error["path"] for error in info.value.errors]
    assert "db.host" in paths


def test_relaxed_boolean(tmp_path: Path) -> None:
    cfg = keydex.load(
        App,
        files=[write(tmp_path, "db:\n  host: h\n")],
        env_prefix="APP_",
        env={"APP_DEBUG": "yes", "APP_DB__TLS": "off"},
    )
    assert cfg.debug is True
    assert cfg.db.tls is False


class Mode(Enum):
    FAST = "fast"
    SLOW = "slow"


@dataclass(frozen=True)
class WithRich:
    mode: Mode
    data_dir: Path


def test_enum_and_path(tmp_path: Path) -> None:
    cfg = keydex.load(
        WithRich,
        files=[write(tmp_path, "mode: slow\ndata_dir: /var/lib/keydex\n")],
        env_prefix="APP_",
        env={},
    )
    assert cfg.mode is Mode.SLOW
    assert cfg.data_dir == Path("/var/lib/keydex")


def test_interpolation_reaches_the_model(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Url:
        host: str
        port: int
        url: str

    cfg = keydex.load(
        Url,
        files=[write(tmp_path, 'host: h\nport: 5432\nurl: "${host}:${port}"\n')],
        env_prefix="APP_",
        env={},
    )
    assert cfg.url == "h:5432"


def test_warns_about_unknown_keys(tmp_path: Path) -> None:
    path = write(tmp_path, "db:\n  host: h\n  prot: 1\n")
    with pytest.warns(keydex.UnknownKeyWarning, match="db.prot"):
        keydex.load(App, files=[path], env_prefix="APP_", env={})


def test_suggests_the_closest_key(tmp_path: Path) -> None:
    path = write(tmp_path, "db:\n  host: h\n  prot: 1\n")
    with pytest.warns(keydex.UnknownKeyWarning, match="db.port"):
        keydex.load(App, files=[path], env_prefix="APP_", env={})


def test_on_unknown_error_turns_the_warning_into_an_error(tmp_path: Path) -> None:
    path = write(tmp_path, "db:\n  host: h\n  prot: 1\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(App, files=[path], env_prefix="APP_", env={}, on_unknown="error")

    assert any(error["path"] == "db.prot" for error in info.value.errors)


def test_on_unknown_ignore_does_not_warn(tmp_path: Path) -> None:
    import warnings

    path = write(tmp_path, "db:\n  host: h\n  prot: 1\n")
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        keydex.load(App, files=[path], env_prefix="APP_", env={}, on_unknown="ignore")

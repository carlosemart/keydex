"""Public Python API tests."""

from __future__ import annotations

from pathlib import Path

import pytest

import keydex

YAML = """\
db:
  host: file.internal
  port: 5432
  url: "${db.host}:${db.port}"
name: "${env:APP_NAME:-anonymous}"
"""


def write(tmp_path: Path, text: str, name: str = "app.yaml") -> str:
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return str(path)


def test_environment_wins_over_file(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={"APP_DB__HOST": "env.internal"})
    assert cfg["db.host"] == "env.internal"
    assert cfg.get("db.port") == 5432


def test_interpolation_and_default(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={})
    assert cfg["db.url"] == "file.internal:5432"
    assert cfg["name"] == "anonymous"


def test_explain_is_structured(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={"APP_DB__HOST": "env.internal"})

    explanation = cfg.explain("db.host")
    assert explanation["key"] == "db.host"
    assert explanation["value"] == "env.internal"
    assert explanation["winner"]["source"] == "env"
    assert explanation["winner"]["tier"] == 2

    (shadowed,) = explanation["shadowed"]
    assert shadowed["source"] == f"file:{path}"
    assert shadowed["line"] == 2
    assert shadowed["col"] == 8


def test_explain_of_an_unknown_key_is_none(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert cfg.explain("does.not.exist") is None


def test_getitem_raises_keyerror(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    with pytest.raises(KeyError):
        cfg["does.not.exist"]


def test_get_with_default(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert cfg.get("does.not.exist", "fallback") == "fallback"


def test_mapping_protocol(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert "db.host" in cfg
    assert "does.not.exist" not in cfg
    assert len(cfg) == 4
    assert cfg.keys() == ["db.host", "db.port", "db.url", "name"]
    assert dict(cfg.items())["db.port"] == 5432


def test_missing_file(tmp_path: Path) -> None:
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([str(tmp_path / "does-not-exist.yaml")])


def test_unknown_extension(tmp_path: Path) -> None:
    path = write(tmp_path, "a: 1\n", name="app.conf")
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([path])


def test_prefix_is_respected(tmp_path: Path) -> None:
    cfg = keydex.load_settings(
        [write(tmp_path, YAML)],
        env_prefix="APP_",
        env={"OTHER_DB__HOST": "must-not-enter"},
    )
    assert cfg["db.host"] == "file.internal"
    assert "other.db.host" not in cfg


def test_parse_error_is_keydexerror(tmp_path: Path) -> None:
    path = write(tmp_path, "a: [unclosed\n")
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([path])

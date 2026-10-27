"""Tests de la API pública de Python."""

from __future__ import annotations

from pathlib import Path

import pytest

import keydex

YAML = """\
db:
  host: file.internal
  port: 5432
  url: "${db.host}:${db.port}"
name: "${env:APP_NAME:-anonimo}"
"""


def write(tmp_path: Path, text: str, name: str = "app.yaml") -> str:
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return str(path)


def test_entorno_gana_al_fichero(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={"APP_DB__HOST": "env.internal"})
    assert cfg["db.host"] == "env.internal"
    assert cfg.get("db.port") == 5432


def test_interpolacion_y_default(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={})
    assert cfg["db.url"] == "file.internal:5432"
    assert cfg["name"] == "anonimo"


def test_explain_es_estructurado(tmp_path: Path) -> None:
    path = write(tmp_path, YAML)
    cfg = keydex.load_settings([path], env_prefix="APP_", env={"APP_DB__HOST": "env.internal"})

    explanation = cfg.explain("db.host")
    assert explanation["key"] == "db.host"
    assert explanation["value"] == "env.internal"
    assert explanation["winner"]["source"] == "env"
    assert explanation["winner"]["tier"] == 2

    (sombreado,) = explanation["shadowed"]
    assert sombreado["source"] == f"file:{path}"
    assert sombreado["line"] == 2
    assert sombreado["col"] == 8


def test_explain_de_clave_inexistente_es_none(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert cfg.explain("no.existe") is None


def test_getitem_lanza_keyerror(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    with pytest.raises(KeyError):
        cfg["no.existe"]


def test_get_con_default(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert cfg.get("no.existe", "por-defecto") == "por-defecto"


def test_protocolo_de_mapeo(tmp_path: Path) -> None:
    cfg = keydex.load_settings([write(tmp_path, YAML)], env_prefix="APP_", env={})
    assert "db.host" in cfg
    assert "no.existe" not in cfg
    assert len(cfg) == 4
    assert cfg.keys() == ["db.host", "db.port", "db.url", "name"]
    assert dict(cfg.items())["db.port"] == 5432


def test_fichero_inexistente(tmp_path: Path) -> None:
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([str(tmp_path / "no-existe.yaml")])


def test_extension_desconocida(tmp_path: Path) -> None:
    path = write(tmp_path, "a: 1\n", name="app.conf")
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([path])


def test_el_prefijo_se_respeta(tmp_path: Path) -> None:
    cfg = keydex.load_settings(
        [write(tmp_path, YAML)],
        env_prefix="APP_",
        env={"OTRA_DB__HOST": "no-debe-entrar"},
    )
    assert cfg["db.host"] == "file.internal"
    assert "otra.db.host" not in cfg


def test_error_de_parseo_es_keydexerror(tmp_path: Path) -> None:
    path = write(tmp_path, "a: [sin cerrar\n")
    with pytest.raises(keydex.KeydexError):
        keydex.load_settings([path])

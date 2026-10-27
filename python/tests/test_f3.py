"""Tests de F3: perfiles, secretos, refresh y CLI."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

import keydex
from keydex import Secret

BASE = "db:\n  host: base\n  port: 5432\n"
PROD = "db:\n  host: prod\n"


def write(tmp_path: Path, name: str, text: str) -> str:
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return str(path)


@dataclass(frozen=True)
class Db:
    user: str
    password: Secret[str]
    host: str = "localhost"


@dataclass(frozen=True)
class App:
    db: Db


def test_el_perfil_gana_al_fichero_base(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-prod.yaml", PROD)

    cfg = keydex.load_settings(files=[base], env_prefix="APP_", env={}, profiles=["prod"])
    assert cfg["db.host"] == "prod"
    # El perfil solo define host: el resto se conserva del base.
    assert cfg["db.port"] == 5432


def test_un_perfil_sin_fichero_se_ignora(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    cfg = keydex.load_settings(files=[base], env_prefix="APP_", env={}, profiles=["nope"])
    assert cfg["db.host"] == "base"


def test_el_ultimo_perfil_de_la_lista_gana(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-a.yaml", "db:\n  host: a\n")
    write(tmp_path, "app-b.yaml", "db:\n  host: b\n")

    cfg = keydex.load_settings(
        files=[base], env_prefix="APP_", env={}, profiles=["a", "b"]
    )
    assert cfg["db.host"] == "b"


def test_los_perfiles_funcionan_en_el_modelo_tipado(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-prod.yaml", PROD)

    cfg = keydex.load(
        App,
        files=[base],
        env_prefix="APP_",
        env={"APP_DB__USER": "u", "APP_DB__PASSWORD": "p"},
        profiles=["prod"],
    )
    assert cfg.db.host == "prod"


def test_el_secreto_no_aparece_en_el_repr(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", "db:\n  user: u\n  password: s3cr3t\n")
    cfg = keydex.load(App, files=[path], env_prefix="APP_", env={})

    assert "s3cr3t" not in repr(cfg)
    assert "s3cr3t" not in str(cfg.db.password)
    assert cfg.db.password.reveal() == "s3cr3t"
    assert isinstance(cfg.db.password, Secret)


def test_indireccion_file_para_secretos(tmp_path: Path) -> None:
    reference = tmp_path / "password"
    reference.write_text("desde-fichero\n", encoding="utf-8")

    cfg = keydex.load(
        App,
        files=[write(tmp_path, "app.yaml", "db:\n  user: u\n")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(reference)},
    )
    assert cfg.db.password.reveal() == "desde-fichero"


def test_file_inexistente_es_error(tmp_path: Path) -> None:
    with pytest.raises(keydex.KeydexError):
        keydex.load(
            App,
            files=[write(tmp_path, "app.yaml", "db:\n  user: u\n")],
            env_prefix="APP_",
            env={"APP_DB__PASSWORD_FILE": str(tmp_path / "no-existe")},
        )


def test_refresh_relee_las_fuentes(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", "k: uno\n")
    cfg = keydex.load_settings(files=[path], env_prefix="APP_", env={})
    assert cfg["k"] == "uno"

    Path(path).write_text("k: dos\n", encoding="utf-8")
    cfg.refresh()
    assert cfg["k"] == "dos"


def test_cli_explain(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "db:\n  host: h\n")
    assert main(["explain", "db.host", "--files", path]) == 0

    salida = capsys.readouterr().out
    assert '"db.host"' in salida
    assert '"source"' in salida


def test_cli_explain_de_clave_desconocida(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "a: 1\n")
    assert main(["explain", "no.existe", "--files", path]) == 2
    assert "desconocida" in capsys.readouterr().err


def test_cli_dump(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "a: 1\nb: dos\n")
    assert main(["dump", "--files", path]) == 0
    assert '"a": 1' in capsys.readouterr().out

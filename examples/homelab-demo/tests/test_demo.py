"""Tests de la demo: perfiles, precedencia, interpolación y secretos."""

from __future__ import annotations

from pathlib import Path

import keydex
import pytest
from main import CONFIG_DIR, load, main, render
from settings import App


def test_dev_por_defecto() -> None:
    app = load(["dev"])
    assert app.env == "dev"
    assert app.db.host == "127.0.0.1"
    assert app.features.metrics is True
    assert app.features.tracing is False


def test_prod_aplica_el_overlay() -> None:
    app = load(["prod"])
    assert app.env == "prod"
    assert app.db.host == "db.internal"
    assert app.db.pool_size == 20
    assert app.backup.keep_days == 30
    assert app.backup.dir == Path("/srv/backups")


def test_la_interpolacion_se_resuelve_despues_del_merge() -> None:
    # `dsn` solo está definido en app.yaml, pero apunta a valores que el perfil cambia.
    app = load(["prod"])
    assert app.db.dsn == "postgres://keydex@db.internal:5432/homelab"


def test_el_entorno_gana_al_perfil() -> None:
    app = keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__HOST": "override.local"},
        profiles=["prod"],
    )
    assert app.db.host == "override.local"
    assert app.env == "prod"


def test_el_secreto_no_aparece_en_el_repr() -> None:
    app = load(["prod"])
    assert "change-me" not in repr(app)
    assert app.db.password.reveal() == "change-me"


def test_render_no_filtra_el_secreto() -> None:
    salida = render(load(["prod"]))
    assert "change-me" not in salida
    assert "Secret('***')" in salida


def test_la_indireccion_file_define_el_secreto(tmp_path: Path) -> None:
    referencia = tmp_path / "password"
    referencia.write_text("desde-fichero\n", encoding="utf-8")

    app = keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(referencia)},
        profiles=["prod"],
    )
    assert app.db.password.reveal() == "desde-fichero"


def test_cli_explain_muestra_el_fichero_ganador(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert main(["--profiles", "prod", "--explain", "db.host"]) == 0
    salida = capsys.readouterr().out
    assert "app-prod.yaml" in salida
    assert '"tier": 3' in salida


def test_cli_devuelve_error_de_validacion(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setenv("APP_DB__PORT", "no-es-un-numero")
    assert main(["--profiles", "dev"]) == 1
    assert "db.port" in capsys.readouterr().err


def test_avisa_si_el_nivel_esta_mal_anidado(tmp_path: Path) -> None:
    # Reproduce el error real: `app.name` en vez de `name` en la raíz.
    config = tmp_path / "app.yaml"
    config.write_text("name: homelab\napp:\n  name: homelab\n", encoding="utf-8")

    with pytest.warns(keydex.UnknownKeyWarning, match="name"):
        keydex.load(App, files=[str(config)], env_prefix="APP_", env={})

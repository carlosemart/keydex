"""Demo tests: profiles, precedence, interpolation and secrets."""

from __future__ import annotations

from pathlib import Path

import keydex
import pytest
from main import CONFIG_DIR, load, main, render
from settings import App


def test_dev_by_default() -> None:
    app = load(["dev"])
    assert app.env == "dev"
    assert app.db.host == "127.0.0.1"
    assert app.features.metrics is True
    assert app.features.tracing is False


def test_prod_applies_the_overlay() -> None:
    app = load(["prod"])
    assert app.env == "prod"
    assert app.db.host == "db.internal"
    assert app.db.pool_size == 20
    assert app.backup.keep_days == 30
    assert app.backup.dir == Path("/srv/backups")


def test_interpolation_runs_after_the_merge() -> None:
    # `dsn` is only defined in app.yaml, but it points at values the profile changes.
    app = load(["prod"])
    assert app.db.dsn == "postgres://keydex@db.internal:5432/homelab"


def test_the_environment_wins_over_the_profile() -> None:
    app = keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__HOST": "override.local"},
        profiles=["prod"],
    )
    assert app.db.host == "override.local"
    assert app.env == "prod"


def test_the_secret_does_not_appear_in_repr() -> None:
    app = load(["prod"])
    assert "change-me" not in repr(app)
    assert app.db.password.reveal() == "change-me"


def test_render_does_not_leak_the_secret() -> None:
    output = render(load(["prod"]))
    assert "change-me" not in output
    assert "Secret('***')" in output


def test_file_indirection_defines_the_secret(tmp_path: Path) -> None:
    reference = tmp_path / "password"
    reference.write_text("from-file\n", encoding="utf-8")

    app = keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(reference)},
        profiles=["prod"],
        secret_files=True,
        secret_root=str(tmp_path),
    )
    assert app.db.password.reveal() == "from-file"


def test_file_indirection_is_off_by_default() -> None:
    app = keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": "/etc/hostname"},
        profiles=["prod"],
    )
    # Without opting in, nothing is read: the base file value is used.
    assert app.db.password.reveal() == "change-me"


def test_secret_outside_the_root_is_rejected(tmp_path: Path) -> None:
    allowed = tmp_path / "allowed"
    allowed.mkdir()
    outside = tmp_path / "outside.txt"
    outside.write_text("nope", encoding="utf-8")

    with pytest.raises(keydex.KeydexError):
        keydex.load(
            App,
            files=[str(CONFIG_DIR / "app.yaml")],
            env_prefix="APP_",
            env={"APP_DB__PASSWORD_FILE": str(outside)},
            secret_files=True,
            secret_root=str(allowed),
        )


def test_explain_redacts_file_backed_secrets(tmp_path: Path) -> None:
    reference = tmp_path / "password"
    reference.write_text("s3cr3t", encoding="utf-8")

    settings = keydex.load_settings(
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(reference)},
        secret_files=True,
        secret_root=str(tmp_path),
    )

    redacted = settings.explain("db.password")
    assert redacted["value"] is None
    assert redacted["redacted"] is True
    assert "s3cr3t" not in repr(dict(settings.items()))

    revealed = settings.explain("db.password", reveal=True)
    assert revealed["value"] == "s3cr3t"


def test_cli_explain_shows_the_winning_file(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert main(["--profiles", "prod", "--explain", "db.host"]) == 0
    output = capsys.readouterr().out
    assert "app-prod.yaml" in output
    assert '"tier": 3' in output


def test_cli_returns_a_validation_error(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setenv("APP_DB__PORT", "not-a-number")
    assert main(["--profiles", "dev"]) == 1
    assert "db.port" in capsys.readouterr().err


def test_warns_when_the_level_is_nested_wrong(tmp_path: Path) -> None:
    # Reproduces the real mistake: `app.name` instead of a root-level `name`.
    config = tmp_path / "app.yaml"
    config.write_text("name: homelab\napp:\n  name: homelab\n", encoding="utf-8")

    with pytest.warns(keydex.UnknownKeyWarning, match="name"):
        keydex.load(App, files=[str(config)], env_prefix="APP_", env={})

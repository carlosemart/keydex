"""Environment tests: one snapshot, two views."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

import keydex


@dataclass(frozen=True)
class Db:
    host: str
    port: int = 5432


@dataclass(frozen=True)
class App:
    db: Db


@dataclass(frozen=True)
class WithPassword:
    db: "PasswordDb"


@dataclass(frozen=True)
class PasswordDb:
    user: str
    password: str


BASE = "db:\n  host: base\n  port: 5432\n"
PROD = "db:\n  host: prod\n"


def write(tmp_path: Path, name: str, text: str) -> str:
    path = tmp_path / name
    path.write_text(text, encoding="utf-8")
    return str(path)


def test_the_model_and_the_provenance_share_one_snapshot(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-prod.yaml", PROD)

    env = keydex.Environment(
        files=[base], env_prefix="APP_", env={}, profiles=["prod"]
    )
    app = env.load(App)

    assert app.db.host == "prod"
    explanation = env.explain("db.host")
    assert explanation is not None
    assert explanation["value"] == "prod"
    assert explanation["winner"]["source"].endswith("app-prod.yaml")


def test_settings_is_a_view_of_the_same_snapshot(tmp_path: Path) -> None:
    env = keydex.Environment(files=[write(tmp_path, "app.yaml", BASE)], env={})
    env.load(App)

    settings = env.settings()
    assert settings["db.host"] == "base"
    assert len(settings) == 2


def test_explain_before_load_is_an_error() -> None:
    with pytest.raises(keydex.KeydexError):
        keydex.Environment().explain("db.host")


def test_refresh_picks_up_a_valid_change(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", BASE)
    env = keydex.Environment(files=[path], env={})
    env.load(App)

    Path(path).write_text("db:\n  host: changed\n  port: 5432\n", encoding="utf-8")
    env.refresh()

    assert env.explain("db.host")["value"] == "changed"


def test_refresh_rejects_an_invalid_configuration_and_keeps_the_old_one(
    tmp_path: Path,
) -> None:
    path = write(tmp_path, "app.yaml", BASE)
    env = keydex.Environment(files=[path], env={})
    env.load(App)

    Path(path).write_text("db:\n  host: h\n  port: not-a-number\n", encoding="utf-8")
    with pytest.raises(keydex.KeydexError):
        env.refresh()

    # The previous snapshot is still served.
    assert env.explain("db.port")["value"] == 5432


def test_a_typed_refresh_needs_a_new_load(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", BASE)
    env = keydex.Environment(files=[path], env={})
    before = env.load(App)

    Path(path).write_text("db:\n  host: changed\n  port: 5432\n", encoding="utf-8")
    env.refresh()
    after = env.load(App)

    assert before.db.host == "base"  # the model is a copy
    assert after.db.host == "changed"


def test_no_prefix_does_not_read_the_process_environment(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Without a prefix, ingesting os.environ would let any variable named like a key
    # (HOST, PORT, USER) override the file behind the user's back.
    monkeypatch.setenv("HOST", "from-the-shell")
    source = write(tmp_path, "app.yaml", "host: from-file\nport: 5432\n")

    settings = keydex.load_settings(files=[source])

    assert settings["host"] == "from-file"


def test_a_prefix_still_reads_the_process_environment(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("APP_HOST", "from-the-shell")
    source = write(tmp_path, "app.yaml", "host: from-file\n")

    settings = keydex.load_settings(files=[source], env_prefix="APP_")

    assert settings["host"] == "from-the-shell"


def test_refresh_without_a_prefix_does_not_read_the_process_environment(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # The initial load ignores os.environ when there is no prefix; refresh must not
    # reintroduce it through the Rust side, which captures the environment directly.
    source = write(tmp_path, "app.yaml", "host: from-file\n")
    settings = keydex.load_settings(files=[source])

    monkeypatch.setenv("HOST", "from-the-shell")
    settings.refresh()

    assert settings["host"] == "from-file"


def test_refresh_expands_a_home_relative_secret_root(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("HOME", str(tmp_path))
    secrets = tmp_path / "secrets"
    secrets.mkdir()
    (secrets / "password").write_text("s3cr3t", encoding="utf-8")
    monkeypatch.setenv("APP_DB_PASSWORD_FILE", "~/secrets/password")

    env = keydex.Environment(
        files=[write(tmp_path, "app.yaml", "db:\n  user: u\n")],
        env_prefix="APP_",
        secret_files=True,
        secret_root="~/secrets",
    )
    settings = env.load(WithPassword)
    assert settings.db.password == "s3cr3t"

    env.refresh()

    assert env.settings()["db"] is None
    assert env.explain("db.password")["redacted"] is True


def test_refresh_of_an_injected_environment_stays_injected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # env={} is a promise that the process environment is irrelevant; refresh must not
    # quietly start reading it.
    monkeypatch.setenv("APP_HOST", "from-the-shell")
    source = write(tmp_path, "app.yaml", "host: from-file\n")

    settings = keydex.load_settings(files=[source], env_prefix="APP_", env={})
    assert settings["host"] == "from-file"

    settings.refresh()

    assert settings["host"] == "from-file"

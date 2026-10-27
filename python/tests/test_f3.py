"""F3 tests: profiles, secrets, refresh and CLI."""

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
    port: int = 5432


@dataclass(frozen=True)
class App:
    db: Db


def test_profile_wins_over_the_base_file(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-prod.yaml", PROD)

    cfg = keydex.load_settings(files=[base], env_prefix="APP_", env={}, profiles=["prod"])
    assert cfg["db.host"] == "prod"
    # The profile only defines host: the rest is kept from the base file.
    assert cfg["db.port"] == 5432


def test_a_profile_without_a_file_is_ignored(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    cfg = keydex.load_settings(
        files=[base], env_prefix="APP_", env={}, profiles=["nope"]
    )
    assert cfg["db.host"] == "base"


def test_the_last_profile_in_the_list_wins(tmp_path: Path) -> None:
    base = write(tmp_path, "app.yaml", BASE)
    write(tmp_path, "app-a.yaml", "db:\n  host: a\n")
    write(tmp_path, "app-b.yaml", "db:\n  host: b\n")

    cfg = keydex.load_settings(
        files=[base], env_prefix="APP_", env={}, profiles=["a", "b"]
    )
    assert cfg["db.host"] == "b"


def test_profiles_work_in_the_typed_model(tmp_path: Path) -> None:
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


def test_the_secret_does_not_appear_in_repr(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", "db:\n  user: u\n  password: s3cr3t\n")
    cfg = keydex.load(App, files=[path], env_prefix="APP_", env={})

    assert "s3cr3t" not in repr(cfg)
    assert "s3cr3t" not in str(cfg.db.password)
    assert cfg.db.password.reveal() == "s3cr3t"
    assert isinstance(cfg.db.password, Secret)


def test_file_indirection_for_secrets(tmp_path: Path) -> None:
    reference = tmp_path / "password"
    reference.write_text("from-file\n", encoding="utf-8")

    cfg = keydex.load(
        App,
        files=[write(tmp_path, "app.yaml", "db:\n  user: u\n")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(reference)},
    )
    assert cfg.db.password.reveal() == "from-file"


def test_missing_file_reference_is_an_error(tmp_path: Path) -> None:
    with pytest.raises(keydex.KeydexError):
        keydex.load(
            App,
            files=[write(tmp_path, "app.yaml", "db:\n  user: u\n")],
            env_prefix="APP_",
            env={"APP_DB__PASSWORD_FILE": str(tmp_path / "does-not-exist")},
        )


def test_refresh_re_reads_the_sources(tmp_path: Path) -> None:
    path = write(tmp_path, "app.yaml", "k: one\n")
    cfg = keydex.load_settings(files=[path], env_prefix="APP_", env={})
    assert cfg["k"] == "one"

    Path(path).write_text("k: two\n", encoding="utf-8")
    cfg.refresh()
    assert cfg["k"] == "two"


def test_cli_explain(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "db:\n  host: h\n")
    assert main(["explain", "db.host", "--files", path]) == 0

    output = capsys.readouterr().out
    assert '"db.host"' in output
    assert '"source"' in output


def test_cli_explain_of_an_unknown_key(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "a: 1\n")
    assert main(["explain", "does.not.exist", "--files", path]) == 2
    assert "unknown key" in capsys.readouterr().err


def test_cli_dump(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    from keydex.__main__ import main

    path = write(tmp_path, "app.yaml", "a: 1\nb: two\n")
    assert main(["dump", "--files", path]) == 0
    assert '"a": 1' in capsys.readouterr().out

"""Typed model tests (F2): coercion, relaxed binding and aggregated errors."""

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Decimal
from enum import Enum
from pathlib import Path
from uuid import UUID

import pytest

import keydex
from keydex import Secret


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


@dataclass(frozen=True)
class CredentialsDb:
    user: str
    password: str


@dataclass(frozen=True)
class SecretDb:
    user: str
    password: Secret[str]


@dataclass(frozen=True)
class SecretApp:
    db: SecretDb


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


def test_an_invalid_enum_is_a_validation_error(tmp_path: Path) -> None:
    path = write(tmp_path, "mode: turbo\ndata_dir: /tmp\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(WithRich, files=[path], env_prefix="APP_", env={})

    assert info.value.errors[0]["path"] == "mode"
    assert "expected one of" in info.value.errors[0]["message"]


def test_an_enum_matches_case_insensitively(tmp_path: Path) -> None:
    path = write(tmp_path, "mode: SLOW\ndata_dir: /tmp\n")
    cfg = keydex.load(WithRich, files=[path], env_prefix="APP_", env={})
    assert cfg.mode is Mode.SLOW


def test_decimal_keeps_its_precision(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Money:
        price: Decimal

    path = write(tmp_path, 'price: "1.10"\n')
    cfg = keydex.load(Money, files=[path], env_prefix="APP_", env={})
    assert cfg.price == Decimal("1.10")
    assert str(cfg.price) == "1.10"


def test_a_decimal_from_a_yaml_number_is_rejected(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Money:
        price: Decimal

    path = write(tmp_path, "price: 1.10\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(Money, files=[path], env_prefix="APP_", env={})
    assert "as text" in info.value.errors[0]["message"]


def test_a_uuid_is_validated(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class WithId:
        id: UUID

    good = write(tmp_path, "id: 550E8400-E29B-41D4-A716-446655440000\n")
    assert keydex.load(WithId, files=[good], env_prefix="APP_", env={}).id == UUID(
        "550e8400-e29b-41d4-a716-446655440000"
    )

    bad = write(tmp_path, "id: nope\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(WithId, files=[bad], env_prefix="APP_", env={})
    assert "UUID" in info.value.errors[0]["message"]


def test_an_optional_default_is_respected(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class WithDefault:
        note: str | None = "keep-me"

    cfg = keydex.load(WithDefault, files=[write(tmp_path, "")], env_prefix="APP_", env={})
    assert cfg.note == "keep-me"


def test_validation_errors_carry_the_position(tmp_path: Path) -> None:
    path = write(tmp_path, "db:\n  host: h\n  port: many\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(App, files=[path], env_prefix="APP_", env={})

    error = info.value.errors[0]
    assert error["path"] == "db.port"
    assert error["file"] == path
    assert error["line"] == 3


def test_a_secret_field_is_redacted_in_explain(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Credentials:
        user: str
        password: Secret[str]

    path = write(tmp_path, "user: u\npassword: s3cr3t\n")
    env = keydex.Environment(files=[path], env={})
    cfg = env.load(Credentials)

    assert cfg.password.reveal() == "s3cr3t"
    assert env.explain("password")["value"] is None
    assert env.explain("password")["redacted"] is True
    assert env.explain("user")["value"] == "u"


def test_refresh_rereads_the_environment(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    @dataclass(frozen=True)
    class Port:
        port: int = 1

    path = write(tmp_path, "port: 1\n")
    env = keydex.Environment(files=[path], env_prefix="APP_")
    env.load(Port)

    monkeypatch.setenv("APP_PORT", "99")
    env.refresh()

    # The snapshot keeps the raw environment string; coercion happens on load.
    assert env.explain("port")["value"] == "99"
    assert env.load(Port).port == 99


def test_the_secret_file_variable_does_not_become_a_key(tmp_path: Path) -> None:
    reference = tmp_path / "pw"
    reference.write_text("s3cr3t", encoding="utf-8")

    settings = keydex.load_settings(
        files=[write(tmp_path, "db:\n  user: u\n")],
        env_prefix="APP_",
        env={"APP_DB__PASSWORD_FILE": str(reference)},
        secret_files=True,
        secret_root=str(tmp_path),
    )

    assert "db.password" in settings
    assert "db.password_file" not in settings


def test_a_free_form_map(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class WithMap:
        labels: dict[str, str] = field(default_factory=dict)

    path = write(tmp_path, "labels:\n  env: prod\n  team: core\n")
    cfg = keydex.load(WithMap, files=[path], env_prefix="APP_", env={})
    assert cfg.labels == {"env": "prod", "team": "core"}


def test_map_values_are_coerced(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class WithLimits:
        limits: dict[str, int] = field(default_factory=dict)

    path = write(tmp_path, "limits:\n  cpu: 2\n  memory: 4\n")
    cfg = keydex.load(WithLimits, files=[path], env_prefix="APP_", env={})
    assert cfg.limits == {"cpu": 2, "memory": 4}


def test_a_bad_map_value_is_a_validation_error(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class WithLimits:
        limits: dict[str, int] = field(default_factory=dict)

    path = write(tmp_path, "limits:\n  cpu: many\n")
    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(WithLimits, files=[path], env_prefix="APP_", env={})
    assert info.value.errors[0]["path"] == "limits.cpu"


def test_indexing_and_dict_are_redacted_but_get_is_raw(tmp_path: Path) -> None:
    settings = keydex.load_settings(
        files=[write(tmp_path, "db:\n  user: u\n  password: s3cr3t\n")],
        env_prefix="APP_",
        env={},
        sensitive=["db.password"],
    )

    assert settings["db.password"] is None
    assert dict(settings)["db.password"] is None
    assert dict(settings.items())["db.password"] is None
    assert settings.get("db.password") == "s3cr3t"


def test_a_container_with_a_secret_is_redacted(tmp_path: Path) -> None:
    env = keydex.Environment(
        files=[write(tmp_path, "db:\n  user: u\n  password: s3cr3t\n")], env={}
    )
    env.load(SecretApp)

    assert env.settings()["db"] is None
    assert env.explain("db")["redacted"] is True
    assert env.explain("db")["value"] is None


def test_a_nested_struct_default_is_respected(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Db:
        host: str

    @dataclass(frozen=True)
    class App:
        db: Db = field(default_factory=lambda: Db(host="fallback"))

    cfg = keydex.load(App, files=[write(tmp_path, "other: x\n")], env_prefix="APP_", env={})
    assert cfg.db.host == "fallback"


def test_a_str_field_from_a_list_is_a_validation_error(tmp_path: Path) -> None:
    @dataclass(frozen=True)
    class Named:
        name: str

    with pytest.raises(keydex.ValidationError) as info:
        keydex.load(
            Named, files=[write(tmp_path, "name:\n  - a\n  - b\n")], env_prefix="APP_", env={}
        )
    assert info.value.errors[0]["path"] == "name"


def test_a_file_variable_marks_the_key_it_binds_to(tmp_path: Path) -> None:
    # Single underscore: relaxed binding maps APP_DB_PASSWORD to db.password. The mark must
    # follow the binding, not a parallel path guess.
    @dataclass(frozen=True)
    class Credentials:
        db: "CredentialsDb"

    reference = tmp_path / "pw"
    reference.write_text("s3cr3t", encoding="utf-8")

    env = keydex.Environment(
        files=[write(tmp_path, "db:\n  user: u\n")],
        env_prefix="APP_",
        env={"APP_DB_PASSWORD_FILE": str(reference)},
        secret_files=True,
        secret_root=str(tmp_path),
    )
    cfg = env.load(Credentials)

    assert cfg.db.password == "s3cr3t"
    assert env.explain("db.password")["redacted"] is True
    assert env.explain("db")["redacted"] is True


def test_a_tuple_annotation_is_rejected(tmp_path: Path) -> None:
    # A tuple would come back as a list: silently wrong, so it is an annotation error.
    @dataclass(frozen=True)
    class Tags:
        tags: tuple[str, ...]

    with pytest.raises(TypeError, match="tuple"):
        keydex.load(Tags, files=[write(tmp_path, "tags: [a]\n")], env_prefix="APP_", env={})

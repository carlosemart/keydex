"""Configuration model of the homelab service.

Everything keydex needs to know lives here: the types and their defaults. Provenance
(who defined each value) is resolved by the engine.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from keydex import Secret


@dataclass(frozen=True)
class Db:
    host: str = "localhost"
    port: int = 5432
    name: str = "homelab"
    user: str = "keydex"
    password: Secret[str] = Secret("")
    pool_size: int = 5
    dsn: str = ""


@dataclass(frozen=True)
class Api:
    base_url: str = "https://api.homelab.local"
    timeout_s: int = 10
    retries: int = 2


@dataclass(frozen=True)
class Features:
    metrics: bool = False
    tracing: bool = False


@dataclass(frozen=True)
class Backup:
    dir: Path = Path("./backups")
    keep_days: int = 7


@dataclass(frozen=True)
class App:
    name: str
    env: str = "dev"
    db: Db = field(default_factory=Db)
    api: Api = field(default_factory=Api)
    features: Features = field(default_factory=Features)
    backup: Backup = field(default_factory=Backup)

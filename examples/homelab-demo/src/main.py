"""Demo: a homelab service loading its configuration with keydex.

Usage:
    python examples/homelab-demo/src/main.py
    python examples/homelab-demo/src/main.py --profiles prod
    python examples/homelab-demo/src/main.py --profiles prod --explain db.host

Relevant environment variables (all with the `APP_` prefix):
    APP_PROFILES=prod                      active profiles
    APP_DB__HOST=db.override               one-off override
    APP_DB__PASSWORD_FILE=/run/secrets/db  password from a file
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import keydex  # noqa: E402
from settings import App  # noqa: E402

CONFIG_DIR = Path(__file__).resolve().parent.parent / "config"


def split_profiles(raw: str) -> list[str]:
    return [part.strip() for part in raw.split(",") if part.strip()]


def load(profiles: list[str]) -> App:
    return keydex.load(
        App,
        files=[str(CONFIG_DIR / "app.yaml")],
        env_prefix="APP_",
        profiles=profiles,
    )


def render(app: App) -> str:
    secret = app.db.password
    return "\n".join(
        [
            f"{app.name} ({app.env})",
            f"  db        {app.db.dsn}  pool={app.db.pool_size}",
            f"  api       {app.api.base_url}  timeout={app.api.timeout_s}s"
            f"  retries={app.api.retries}",
            f"  backup    {app.backup.dir}  retention={app.backup.keep_days}d",
            f"  features  metrics={app.features.metrics} tracing={app.features.tracing}",
            f"  secret    {secret!r} ({len(secret.reveal())} bytes loaded)",
        ]
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="keydex demo on a homelab service")
    parser.add_argument(
        "--profiles",
        default=os.environ.get("APP_PROFILES", "dev"),
        help="comma separated profiles (default: APP_PROFILES or 'dev')",
    )
    parser.add_argument("--explain", metavar="KEY", help="show the provenance of a key")
    args = parser.parse_args(argv)

    profiles = split_profiles(args.profiles)

    if args.explain:
        settings = keydex.load_settings(
            files=[str(CONFIG_DIR / "app.yaml")],
            env_prefix="APP_",
            profiles=profiles,
        )
        explanation = settings.explain(args.explain)
        if explanation is None:
            print(f"unknown key: {args.explain}", file=sys.stderr)
            return 2
        print(json.dumps(explanation, indent=2, ensure_ascii=False))
        return 0

    try:
        app = load(profiles)
    except keydex.ValidationError as error:
        print("invalid configuration:", file=sys.stderr)
        for item in error.errors:
            print(f"  {item['path']}: {item['message']}", file=sys.stderr)
        return 1

    print(render(app))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

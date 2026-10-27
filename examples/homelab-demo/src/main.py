"""Demo: un servicio de homelab cargando su configuración con keydex.

Uso:
    python examples/homelab-demo/src/main.py
    python examples/homelab-demo/src/main.py --profiles prod
    python examples/homelab-demo/src/main.py --profiles prod --explain db.host

Variables de entorno relevantes (todas con prefijo `APP_`):
    APP_PROFILES=prod                     perfiles activos
    APP_DB__HOST=db.override              override puntual
    APP_DB__PASSWORD_FILE=/run/secrets/db contraseña desde fichero
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
            f"  reintentos={app.api.retries}",
            f"  backup    {app.backup.dir}  retención={app.backup.keep_days}d",
            f"  features  metrics={app.features.metrics} tracing={app.features.tracing}",
            f"  secreto   {secret!r} ({len(secret.reveal())} bytes cargados)",
        ]
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="demo de keydex en un servicio de homelab")
    parser.add_argument(
        "--profiles",
        default=os.environ.get("APP_PROFILES", "dev"),
        help="perfiles separados por comas (por defecto: APP_PROFILES o 'dev')",
    )
    parser.add_argument("--explain", metavar="CLAVE", help="muestra la procedencia de una clave")
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
            print(f"clave desconocida: {args.explain}", file=sys.stderr)
            return 2
        print(json.dumps(explanation, indent=2, ensure_ascii=False))
        return 0

    try:
        app = load(profiles)
    except keydex.ValidationError as error:
        print("configuración inválida:", file=sys.stderr)
        for item in error.errors:
            print(f"  {item['path']}: {item['message']}", file=sys.stderr)
        return 1

    print(render(app))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

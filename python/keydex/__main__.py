"""CLI de keydex: `python -m keydex`."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence

import keydex


def _common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--files", nargs="+", default=[], help="ficheros de configuración")
    parser.add_argument("--env-prefix", default="", help="prefijo de las variables de entorno")
    parser.add_argument("--profiles", nargs="*", default=[], help="perfiles activos")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m keydex")
    sub = parser.add_subparsers(dest="command", required=True)

    explain = sub.add_parser("explain", help="muestra la procedencia de una clave")
    explain.add_argument("key")
    _common(explain)

    dump = sub.add_parser("dump", help="vuelca la configuración resuelta en JSON")
    _common(dump)

    args = parser.parse_args(argv)

    try:
        settings = keydex.load_settings(
            files=args.files,
            env_prefix=args.env_prefix,
            profiles=args.profiles,
        )
    except keydex.KeydexError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if args.command == "explain":
        explanation = settings.explain(args.key)
        if explanation is None:
            print(f"clave desconocida: {args.key}", file=sys.stderr)
            return 2
        print(json.dumps(explanation, indent=2, ensure_ascii=False))
        return 0

    print(json.dumps(dict(settings.items()), indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

"""keydex CLI: `python -m keydex`."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence

import keydex


def _common(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--files", nargs="+", default=[], help="configuration files")
    parser.add_argument("--env-prefix", default="", help="environment variable prefix")
    parser.add_argument("--profiles", nargs="*", default=[], help="active profiles")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="python -m keydex")
    sub = parser.add_subparsers(dest="command", required=True)

    explain = sub.add_parser("explain", help="shows the provenance of a key")
    explain.add_argument("key")
    _common(explain)

    dump = sub.add_parser("dump", help="dumps the resolved configuration as JSON")
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
            print(f"unknown key: {args.key}", file=sys.stderr)
            return 2
        print(json.dumps(explanation, indent=2, ensure_ascii=False))
        return 0

    print(json.dumps(dict(settings.items()), indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

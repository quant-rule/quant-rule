#!/usr/bin/env python3
"""Validate one improvement run manifest against quantrule.improvement-run/v1.

The command is read-only. It prints ``ok <run_id>`` on success and every schema
defect to stderr on failure, then exits 1. Partition names are read from
``data/partitions.json`` so a manifest cannot cite an undeclared partition.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from common import ToolError, read_json, validate_manifest


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="validate.py",
        description=(
            "Validate a quantrule.improvement-run/v1 manifest. The check covers "
            "required fields, run-id shape and tier prefix, UTC creation time, "
            "the changed/frozen split, the declared partition, and holdout use."
        ),
        epilog=(
            "Example: python3 tools/improvement/validate.py "
            "data/improvement/2026-11-01/MODEL-2026-11-01-01/manifest.json"
        ),
    )
    parser.add_argument("manifest", help="path to manifest.json")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    path = Path(args.manifest)
    if not path.is_file():
        print(f"error: manifest not found: {path}", file=sys.stderr)
        return 1
    try:
        manifest = read_json(path)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    errors = validate_manifest(manifest)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"ok {manifest['run_id']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

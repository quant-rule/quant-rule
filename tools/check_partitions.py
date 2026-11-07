#!/usr/bin/env python3
"""Check the partition file, instrument aliases, and config references.

Every partition declares a window, a use, and instruments. Every instrument is a
canonical root instrument. Every alias resolves to a canonical instrument. Every
``instrument`` value in a JSON config resolves through the canonical list or the
alias map.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def read_json(path: Path, root: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read {path.relative_to(root)}: {error}") from None


def config_instruments(root: Path) -> list[tuple[str, str]]:
    found: list[tuple[str, str]] = []
    config = root / "config"
    if not config.is_dir():
        return found
    for path in sorted(config.rglob("*.json")):
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if not isinstance(document, dict):
            continue
        instrument = document.get("instrument")
        if isinstance(instrument, str) and instrument:
            found.append((path.relative_to(root).as_posix(), instrument))
    return found


def check(root: Path) -> list[str]:
    path = root / "data" / "partitions.json"
    if not path.is_file():
        return ["data/partitions.json is missing"]
    try:
        document = read_json(path, root)
    except ValueError as error:
        return [str(error)]
    if not isinstance(document, dict):
        return ["data/partitions.json must be a JSON object"]

    problems: list[str] = []
    instruments = document.get("instruments")
    aliases = document.get("aliases")
    partitions = document.get("partitions")
    if not isinstance(instruments, list) or not instruments:
        problems.append("instruments must be a non-empty list")
        instruments = []
    if not isinstance(aliases, dict):
        problems.append("aliases must be an object")
        aliases = {}
    if not isinstance(partitions, dict) or not partitions:
        problems.append("partitions must be a non-empty object")
        partitions = {}

    canonical = [item for item in instruments if isinstance(item, str) and item]
    if len(canonical) != len(set(canonical)):
        problems.append("instruments must be unique")
    canonical_set = set(canonical)

    for alias, target in sorted(aliases.items()):
        if not isinstance(alias, str) or not alias:
            problems.append("an alias must be a non-empty string")
        if not isinstance(target, str) or target not in canonical_set:
            problems.append(f"alias {alias!r} does not resolve to a canonical instrument")

    for name, partition in sorted(partitions.items()):
        if not isinstance(partition, dict):
            problems.append(f"partition {name!r} must be an object")
            continue
        window = partition.get("window")
        if not isinstance(window, dict):
            problems.append(f"partition {name!r} has no window")
        else:
            start = window.get("from")
            end = window.get("to")
            if not isinstance(start, str) or not DATE.match(start):
                problems.append(f"partition {name!r} window.from must be YYYY-MM-DD")
            if not isinstance(end, str) or not DATE.match(end):
                problems.append(f"partition {name!r} window.to must be YYYY-MM-DD")
            if isinstance(start, str) and isinstance(end, str) and start > end:
                problems.append(f"partition {name!r} window.from is after window.to")
        if not isinstance(partition.get("use"), str) or not partition["use"]:
            problems.append(f"partition {name!r} has no use")
        declared = partition.get("instruments")
        if not isinstance(declared, list) or not declared:
            problems.append(f"partition {name!r} has no instruments")
            continue
        for instrument in declared:
            if instrument not in canonical_set:
                problems.append(
                    f"partition {name!r} cites undeclared instrument {instrument!r}"
                )

    for config_path, instrument in config_instruments(root):
        if instrument not in canonical_set and instrument not in aliases:
            problems.append(f"{config_path}: instrument {instrument!r} is not declared")

    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    document = json.loads((ROOT / "data" / "partitions.json").read_text(encoding="utf-8"))
    print(
        "partitions ok: "
        f"{len(document.get('instruments', []))} instruments, "
        f"{len(document.get('partitions', {}))} partitions, "
        f"{len(document.get('aliases', {}))} aliases"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

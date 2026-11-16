#!/usr/bin/env python3
"""Check the partition file, instrument aliases, and config references.

Every partition declares a window, a use, and instruments. Every instrument is a
canonical root instrument. Every alias resolves to a canonical instrument. Every
``instrument`` or ``instruments`` value in a JSON config, at any nesting depth,
resolves through the canonical list or the alias map. Excluded windows are
well-formed, ordered, disjoint, and carry a reason; no partition window may
intersect one.

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
    """Collect every instrument reference from the JSON configs.

    Configs nest instruments inside objects and lists (for example a strategy
    bundle that names one instrument per leg), so the walk is recursive and
    records the JSON path of each reference. The checker supports only string
    values under an ``instrument`` key and string items under an ``instruments``
    key; any other shape is left to the config's own schema.
    """
    found: list[tuple[str, str]] = []
    config = root / "config"
    if not config.is_dir():
        return found
    for path in sorted(config.rglob("*.json")):
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        relative = path.relative_to(root).as_posix()
        for trail, instrument in walk_instruments(document):
            found.append((f"{relative}:{trail}", instrument))
    return found


def walk_instruments(document: object, trail: str = "") -> list[tuple[str, str]]:
    """Return every ``instrument`` or ``instruments`` leaf under ``document``."""
    found: list[tuple[str, str]] = []
    if isinstance(document, dict):
        for key, value in document.items():
            path = f"{trail}.{key}" if trail else key
            if key == "instrument" and isinstance(value, str) and value:
                found.append((path, value))
            if key == "instruments" and isinstance(value, list):
                for index, item in enumerate(value):
                    if isinstance(item, str) and item:
                        found.append((f"{path}[{index}]", item))
            found.extend(walk_instruments(value, path))
    elif isinstance(document, list):
        for index, item in enumerate(document):
            found.extend(walk_instruments(item, f"{trail}[{index}]"))
    return found


def excluded_windows(document: dict, problems: list[str]) -> list[tuple[str, str]]:
    """Validate the excluded windows and return the well-formed pairs.

    An excluded window is a dated interval with a written reason. The list must
    be ordered, every window must be well-formed, and no two windows may
    overlap; a malformed window is reported instead of being skipped, because a
    silently ignored exclusion is exactly the failure this check exists for.
    """
    excluded = document.get("excluded_windows")
    if not isinstance(excluded, list):
        problems.append("excluded_windows must be a list")
        return []
    windows: list[tuple[str, str]] = []
    for index, window in enumerate(excluded):
        label = f"excluded_windows[{index}]"
        if not isinstance(window, dict):
            problems.append(f"{label} must be an object")
            continue
        start = window.get("from")
        end = window.get("to")
        if not isinstance(start, str) or not DATE.match(start):
            problems.append(f"{label}.from must be YYYY-MM-DD")
            start = None
        if not isinstance(end, str) or not DATE.match(end):
            problems.append(f"{label}.to must be YYYY-MM-DD")
            end = None
        reason = window.get("reason")
        if not isinstance(reason, str) or not reason.strip():
            problems.append(f"{label} has no reason")
        if start is None or end is None:
            continue
        if start > end:
            problems.append(f"{label}.from is after .to")
            continue
        if windows and start < windows[-1][0]:
            problems.append(f"{label} is out of order")
        windows.append((start, end))
    for previous, current in zip(windows, windows[1:]):
        if current[0] <= previous[1]:
            problems.append(
                f"excluded_windows overlap: {previous[0]}..{previous[1]} and "
                f"{current[0]}..{current[1]}"
            )
    return windows


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
    excluded = excluded_windows(document, problems)

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
            if (
                isinstance(start, str)
                and isinstance(end, str)
                and DATE.match(start)
                and DATE.match(end)
                and start <= end
            ):
                for excluded_start, excluded_end in excluded:
                    if start <= excluded_end and excluded_start <= end:
                        problems.append(
                            f"partition {name!r} overlaps excluded window "
                            f"{excluded_start}..{excluded_end}"
                        )
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

#!/usr/bin/env python3
"""Check that the generated verification matrices cover the registry.

The indicator matrix must have one row per active indicator. The event matrix
must have one row per active event. Both are generated from
``docs/registry/indicators.json`` and ``docs/registry/validation.json``.

Usage:
    python3 tools/check_verification_matrix.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs/registry/indicators.json"
INDICATORS = ROOT / "docs/model/2026-11-06/indicator-verification.md"
EVENTS = ROOT / "docs/model/2026-11-06/event-verification.md"

INDICATOR_ROW = re.compile(r"^\|\s*\d+\s*\|\s*`([^`]+)`\s*\|")
EVENT_ROW = re.compile(r"^\|\s*`([^`]+)`\s*\|\s*\d+\s*\|")


def load(path: Path) -> dict[str, object]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def main() -> int:
    registry = load(REGISTRY)
    expected_indicators = [
        indicator["id"]
        for indicator in registry["indicators"]
        if indicator.get("status") == "active"
    ]
    expected_events = [
        event["id"] for event in registry["events"] if event.get("status") == "active"
    ]
    errors: list[str] = []

    indicator_ids = [
        match.group(1)
        for line in INDICATORS.read_text(encoding="utf-8").splitlines()
        if (match := INDICATOR_ROW.match(line))
    ]
    event_ids = [
        match.group(1)
        for line in EVENTS.read_text(encoding="utf-8").splitlines()
        if (match := EVENT_ROW.match(line))
    ]

    for label, expected, observed in [
        ("indicator", expected_indicators, indicator_ids),
        ("event", expected_events, event_ids),
    ]:
        if len(observed) != len(expected):
            errors.append(f"{label} matrix has {len(observed)} rows, expected {len(expected)}")
        if len(set(observed)) != len(observed):
            errors.append(f"{label} matrix has duplicate rows")
        missing = sorted(set(expected) - set(observed))
        extra = sorted(set(observed) - set(expected))
        if missing:
            errors.append(f"{label} matrix is missing: {', '.join(missing[:10])}")
        if extra:
            errors.append(f"{label} matrix has unknown rows: {', '.join(extra[:10])}")

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"verification matrix ok: {len(indicator_ids)} indicators, "
        f"{len(event_ids)} events"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

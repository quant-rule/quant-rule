#!/usr/bin/env python3
"""Check that every registered event code has a production detector reference.

The registry is the contract. ``codes.rs`` names every stable event code. A code
is emitted directly when a detector names its constant, or arithmetically when a
detector names a family base constant and adds the subtype offset.

Tests and fixture files are excluded: a test reference is not an emitter.

Usage:
    python3 tools/check_event_emitters.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CODES = ROOT / "crates/quantrule-model/src/events/codes.rs"
EVENTS = ROOT / "crates/quantrule-model/src/events"
REGISTRY = ROOT / "docs/registry/indicators.json"

CONSTANT = re.compile(r"pub const ([A-Z0-9_]+): u16 = (\d+);")


def load_registry() -> dict[int, dict[str, object]]:
    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    return {event["code"]: event for event in registry["events"]}


def production_sources() -> list[str]:
    sources = []
    for path in EVENTS.rglob("*.rs"):
        if path.name == "codes.rs" or path.name == "tests.rs":
            continue
        if "tests" in path.parts:
            continue
        sources.append(path.read_text(encoding="utf-8"))
    return sources


def main() -> int:
    registry = load_registry()
    codes = {
        name: int(number)
        for name, number in CONSTANT.findall(CODES.read_text(encoding="utf-8"))
    }
    sources = production_sources()
    referenced = {
        name for name in codes if any(re.search(rf"\b{re.escape(name)}\b", text) for text in sources)
    }
    missing: list[str] = []
    for code, event in sorted(registry.items()):
        names = [name for name, number in codes.items() if number == code]
        if any(name in referenced for name in names):
            continue
        arithmetic = False
        for base_name in referenced:
            base_code = codes[base_name]
            base = registry.get(base_code)
            if base is None:
                continue
            if (
                base.get("event_type") == event.get("event_type")
                and base.get("source") == event.get("source")
                and code >= base_code
                and code - base_code == int(event["subtype"]) - int(base["subtype"])
            ):
                arithmetic = True
                break
        if not arithmetic:
            missing.append(f"{event['id']} ({names[0] if names else 'no constant'}, code {code})")
    if missing:
        for item in missing:
            print(f"error: no production emitter reference: {item}", file=sys.stderr)
        return 1
    print(f"event emitters ok: {len(registry)} registered codes referenced or family-computed")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Check that every active registry indicator and event has a validation claim.

The validation file is the bridge between the documentation registry and the
verification harness. This checker fails when an active indicator or event has
no claim, when a claim names an unknown or inactive registry entry, or when a
signal/context event has no primary outcome.

Usage:
    python3 tools/check_validation.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs/registry/indicators.json"
VALIDATION = ROOT / "docs/registry/validation.json"

CLAIM_KEYS = {
    "id", "kind", "construct_tests", "primary_outcome",
    "horizon_bars", "target_atr", "stop_atr", "baseline",
    "predictive_status",
}
REQUIRED = {"id", "kind", "construct_tests", "primary_outcome", "predictive_status"}
KINDS = {"indicator", "event"}
STATUSES = {"pending", "pass", "no_edge", "defect", "inconclusive"}
OUTCOMES = {
    "path_directional", "structure", "reversal", "volatility",
    "participation", "progression", "trend_strength", "none",
}


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def check(registry: dict[str, Any], validation: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if validation.get("schema") != "quantrule.validation/v1":
        errors.append("validation schema must be quantrule.validation/v1")
    claims = validation.get("claims")
    if not isinstance(claims, list):
        return errors + ["validation.claims must be an array"]

    active_indicators = {
        indicator["id"]
        for indicator in registry.get("indicators", [])
        if indicator.get("status") == "active"
    }
    active_events = {
        event["id"]
        for event in registry.get("events", [])
        if event.get("status") == "active"
    }
    seen: set[str] = set()
    claimed_indicators: set[str] = set()
    claimed_events: set[str] = set()

    for index, claim in enumerate(claims):
        label = f"claims[{index}]"
        if not isinstance(claim, dict):
            errors.append(f"{label} must be an object")
            continue
        missing = REQUIRED - set(claim)
        extra = set(claim) - CLAIM_KEYS
        if missing:
            errors.append(f"{label} missing keys: {sorted(missing)}")
        if extra:
            errors.append(f"{label} unknown keys: {sorted(extra)}")
        ident = claim.get("id")
        if not isinstance(ident, str) or not ident:
            errors.append(f"{label}.id must be a non-empty string")
            continue
        if ident in seen:
            errors.append(f"{label}.id duplicate: {ident}")
        seen.add(ident)
        kind = claim.get("kind")
        if kind not in KINDS:
            errors.append(f"{label}.kind is invalid")
            continue
        if kind == "indicator":
            claimed_indicators.add(ident)
            if ident not in active_indicators:
                errors.append(f"{label} names an unknown or inactive indicator: {ident}")
        else:
            claimed_events.add(ident)
            if ident not in active_events:
                errors.append(f"{label} names an unknown or inactive event: {ident}")
        tests = claim.get("construct_tests")
        if not isinstance(tests, list) or not tests or any(
            not isinstance(test, str) or not test for test in tests
        ):
            errors.append(f"{label}.construct_tests must be a non-empty string array")
        outcome = claim.get("primary_outcome")
        if outcome not in OUTCOMES:
            errors.append(f"{label}.primary_outcome is invalid")
        if kind == "event" and outcome == "none":
            errors.append(f"{label} is an event with no primary outcome")
        if claim.get("predictive_status") not in STATUSES:
            errors.append(f"{label}.predictive_status is invalid")
        if kind == "event":
            for field in ("horizon_bars", "target_atr", "stop_atr"):
                if not isinstance(claim.get(field), (int, float)) or claim[field] <= 0:
                    errors.append(f"{label}.{field} must be positive")
            if not isinstance(claim.get("baseline"), str) or not claim["baseline"]:
                errors.append(f"{label}.baseline must be a non-empty string")

    for ident in sorted(active_indicators - claimed_indicators):
        errors.append(f"active indicator has no validation claim: {ident}")
    for ident in sorted(active_events - claimed_events):
        errors.append(f"active event has no validation claim: {ident}")
    return errors


def main() -> int:
    registry = load(REGISTRY)
    validation = load(VALIDATION)
    errors = check(registry, validation)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"validation claims ok: {len(validation['claims'])} claims "
        f"for {len(registry['indicators'])} indicators and {len(registry['events'])} events"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

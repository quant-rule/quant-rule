#!/usr/bin/env python3
"""Fail when the client or Rust artifact label table drifts from the registry."""

from __future__ import annotations

import re
import sys
from pathlib import Path

from generate_labels import REGISTRY, check, load

ROOT = Path(__file__).resolve().parent.parent
RUST = ROOT / "crates/quantrule-app/src/artifact/labels.rs"


def arms(source: str, name: str) -> tuple[dict[int, str], dict[tuple[int, int], str]]:
    """Read the scalar and pair arms of one label function."""
    start = source.index(f"pub fn {name}(")
    end = source.index("\n}", start)
    block = source[start:end]
    scalar: dict[int, str] = {}
    pair: dict[tuple[int, int], str] = {}
    for match in re.finditer(
        r'^\s*(?:\((\d+),\s*(\d+)\)|(-?\d+))\s*=>\s*"([^"]+)",',
        block,
        re.MULTILINE,
    ):
        first, second, single, label = match.groups()
        if first is not None:
            pair[(int(first), int(second))] = label
        else:
            scalar[int(single)] = label
    return scalar, pair


def registry_map(codes: dict[str, object], key: str) -> dict[int, str]:
    values = codes.get(key)
    if not isinstance(values, dict):
        raise SystemExit(f"error: the registry has no {key} code map")
    return {int(code): str(label) for code, label in values.items()}


def compare(name: str, expected: dict[object, str], actual: dict[object, str]) -> list[str]:
    problems = []
    for code in sorted(expected):
        if code not in actual:
            problems.append(f"{name}{code} is missing")
        elif actual[code] != expected[code]:
            problems.append(
                f"{name}{code} is {actual[code]!r}; the registry says {expected[code]!r}"
            )
    for code in sorted(set(actual) - set(expected)):
        problems.append(f"{name}{code} is not in the registry")
    return problems


def rust_problems(source: str, registry: dict[str, object]) -> list[str]:
    codes = registry.get("codes")
    if not isinstance(codes, dict):
        raise SystemExit("error: the registry has no codes object")
    problems: list[str] = []
    for function, key in [
        ("event_type_label", "event_types"),
        ("source_label", "sources"),
        ("stage_label", "stages"),
        ("lifecycle_label", "lifecycles"),
        ("scale_label", "scales"),
        ("target_basis_label", "target_bases"),
    ]:
        actual, _ = arms(source, function)
        problems.extend(compare(function, registry_map(codes, key), actual))

    events = registry.get("events")
    if not isinstance(events, list):
        raise SystemExit("error: the registry has no events list")
    expected_events = {
        int(event["code"]): str(event["id"]) for event in events if "code" in event and "id" in event
    }
    actual_events, _ = arms(source, "event_name")
    problems.extend(compare("event_name", expected_events, actual_events))

    expected_subtypes: dict[tuple[int, int], str] = {}
    for type_code, key in [
        (1, "level_subtypes"),
        (2, "divergence_subtypes"),
        (3, "signal_subtypes"),
    ]:
        for subtype, label in registry_map(codes, key).items():
            expected_subtypes[(type_code, subtype)] = label
    _, actual_subtypes = arms(source, "event_subtype_label")
    problems.extend(compare("event_subtype_label", expected_subtypes, actual_subtypes))
    return problems


def check_rust() -> int:
    problems = rust_problems(RUST.read_text(encoding="utf-8"), load(REGISTRY))
    if problems:
        for problem in problems:
            print(f"error: {RUST.relative_to(ROOT)}: {problem}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    status = check()
    if status == 0:
        status = check_rust()
    if status == 0:
        print("labels ok: the client table and Rust artifact labels match the registry")
    sys.exit(status)

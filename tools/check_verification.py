#!/usr/bin/env python3
"""Check the per-row verification manifest.

Every active indicator and event must have all evidence classes. A ``row``
scope must name the row's indicator or event in the test name. A ``family``
scope may name a family test. A ``tool`` scope must name a known repository
gate. ``n/a`` carries no test.

Usage:
    python3 tools/check_verification.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from check_validation import rust_function_names

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs/registry/indicators.json"
MANIFEST = ROOT / "docs/registry/verification.json"
TOOLS = {"check_registry.py", "check_validation.py", "check_event_emitters.py"}
CLASSES = [
    "formula",
    "causality",
    "determinism",
    "golden",
    "edge",
    "availability",
    "no_double_count",
    "precedence",
    "math",
    "spec",
    "emitter",
]
SCOPES = {"row", "family", "tool", "n/a"}


def load(path: Path) -> dict[str, object]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def main() -> int:
    registry = load(REGISTRY)
    manifest = load(MANIFEST)
    rust_functions = rust_function_names()
    errors: list[str] = []
    if manifest.get("schema") != "quantrule.verification/v1":
        errors.append("manifest schema must be quantrule.verification/v1")

    for kind, active in [
        ("indicators", [item["id"] for item in registry["indicators"] if item.get("status") == "active"]),
        ("events", [item["id"] for item in registry["events"] if item.get("status") == "active"]),
    ]:
        entries = manifest.get(kind)
        if not isinstance(entries, list):
            errors.append(f"{kind} must be an array")
            continue
        seen = set()
        for entry in entries:
            ident = entry.get("id")
            if ident in seen:
                errors.append(f"{kind} duplicate entry: {ident}")
            seen.add(ident)
            if kind == "indicators" and entry.get("kind") != "indicator":
                errors.append(f"{ident}: kind must be indicator")
            if kind == "events" and entry.get("kind") != "event":
                errors.append(f"{ident}: kind must be event")
            evidence = entry.get("evidence")
            if not isinstance(evidence, dict):
                errors.append(f"{ident}: evidence must be an object")
                continue
            for name in CLASSES:
                item = evidence.get(name)
                if not isinstance(item, dict):
                    errors.append(f"{ident}: missing evidence class {name}")
                    continue
                scope = item.get("scope")
                tests = item.get("tests")
                if scope not in SCOPES:
                    errors.append(f"{ident}.{name}: invalid scope {scope!r}")
                    continue
                if not isinstance(tests, list) or any(not isinstance(test, str) or not test for test in tests):
                    errors.append(f"{ident}.{name}: tests must be a non-empty string array")
                    continue
                if scope == "n/a":
                    if tests:
                        errors.append(f"{ident}.{name}: n/a must not carry tests")
                    continue
                if scope == "tool":
                    unknown = [test for test in tests if test not in TOOLS]
                    if unknown:
                        errors.append(f"{ident}.{name}: unknown tool gate {unknown}")
                    continue
                missing = [test for test in tests if test.rsplit("::", 1)[-1] not in rust_functions]
                if missing:
                    errors.append(f"{ident}.{name}: missing Rust test {missing}")
                if scope == "row":
                    suffix = str(ident).split(".")[-1].replace("_", "").lower()
                    if not any(suffix in test.replace("_", "").lower() for test in tests):
                        errors.append(f"{ident}.{name}: row scope does not name the row")
        missing = sorted(set(active) - seen)
        extra = sorted(seen - set(active))
        if missing:
            errors.append(f"{kind} missing entries: {', '.join(missing[:10])}")
        if extra:
            errors.append(f"{kind} unknown entries: {', '.join(extra[:10])}")

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    scopes = {scope: 0 for scope in SCOPES}
    for kind in ("indicators", "events"):
        for entry in manifest[kind]:
            for item in entry["evidence"].values():
                scopes[item["scope"]] += 1
    print(
        "verification ok: "
        f"{len(manifest['indicators'])} indicators, {len(manifest['events'])} events; "
        f"row={scopes['row']} family={scopes['family']} tool={scopes['tool']} n/a={scopes['n/a']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Check the per-row verification manifest.

Every active indicator and event must have all evidence classes. A ``row``
scope must name the row's indicator or event in the test name. A ``family``
scope may name a family test. A ``tool`` scope must name a known repository
gate. ``n/a`` carries no test, and it is rejected for the four classes every
active indicator and event must carry: causality, determinism, golden, and
edge. Those classes prove the model-layer evidence contract, so an inapplicable
scope would let a row claim coverage without a test.

Usage:
    python3 tools/check_verification.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from check_validation import rust_function_names

ROOT = Path(__file__).resolve().parent.parent


def rust_event_test_names() -> set[str]:
    """Collect test functions whose body constructs or reads an event batch.

    A readings-only test cannot prove an event edge case. The scan is
    deliberately conservative: the body must name ``EventRecord`` or call
    ``.events()``. The function body ends at the first closing brace at the
    function's own indentation, which is how the test modules are formatted.
    """
    names: set[str] = set()
    for path in (ROOT / "crates").rglob("*.rs"):
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        bodies: dict[str, str] = {}
        for index, line in enumerate(lines):
            match = re.match(r"(\s*)fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", line)
            if match is None:
                continue
            indent = len(match.group(1))
            body = [line]
            for later in lines[index + 1 :]:
                body.append(later)
                if later.strip() == "}" and len(later) - len(later.lstrip()) == indent:
                    break
            bodies[match.group(2)] = "\n".join(body)
        for index, line in enumerate(lines):
            match = re.match(r"(\s*)fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", line)
            if match is None:
                continue
            prefix = "\n".join(lines[max(0, index - 3) : index])
            if "#[test]" not in prefix:
                continue
            text = bodies.get(match.group(2), "")
            called = set(re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", text))
            expanded = "\n".join([text, *(bodies[name] for name in called if name in bodies)])
            if "EventRecord" in expanded or ".events()" in expanded:
                names.add(match.group(2))
    return names
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
REQUIRED_CLASSES = ("causality", "determinism", "golden", "edge")


def load(path: Path) -> dict[str, object]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def check(
    root: Path,
    rust_functions: set[str] | None = None,
    rust_event_functions: set[str] | None = None,
) -> list[str]:
    """Return every problem in the verification manifest under ``root``.

    ``rust_functions`` and ``rust_event_functions`` let a test supply the known
    test names instead of scanning the repository.
    """
    registry = load(root / "docs/registry/indicators.json")
    manifest = load(root / "docs/registry/verification.json")
    functions = rust_function_names() if rust_functions is None else rust_functions
    event_functions = (
        rust_event_test_names()
        if rust_event_functions is None
        else rust_event_functions
    )
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
                    if name in REQUIRED_CLASSES:
                        errors.append(f"{ident}.{name}: {name} evidence cannot be n/a")
                    if tests:
                        errors.append(f"{ident}.{name}: n/a must not carry tests")
                    continue
                if scope == "tool":
                    unknown = [test for test in tests if test not in TOOLS]
                    if unknown:
                        errors.append(f"{ident}.{name}: unknown tool gate {unknown}")
                    continue
                missing = [test for test in tests if test.rsplit("::", 1)[-1] not in functions]
                if missing:
                    errors.append(f"{ident}.{name}: missing Rust test {missing}")
                if scope == "row":
                    suffix = str(ident).split(".")[-1].replace("_", "").lower()
                    if not any(suffix in test.replace("_", "").lower() for test in tests):
                        errors.append(f"{ident}.{name}: row scope does not name the row")
                if kind == "events" and name == "edge" and scope != "tool":
                    not_event = [
                        test
                        for test in tests
                        if test.rsplit("::", 1)[-1] not in event_functions
                    ]
                    if not_event:
                        errors.append(
                            f"{ident}.{name}: edge evidence does not construct events {not_event}"
                        )
        missing = sorted(set(active) - seen)
        extra = sorted(seen - set(active))
        if missing:
            errors.append(f"{kind} missing entries: {', '.join(missing[:10])}")
        if extra:
            errors.append(f"{kind} unknown entries: {', '.join(extra[:10])}")

    return errors


def main() -> int:
    errors = check(ROOT, rust_event_functions=rust_event_test_names())
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    manifest = load(MANIFEST)
    scopes = {scope: 0 for scope in SCOPES}
    for kind in ("indicators", "events"):
        for entry in manifest[kind]:
            for item in entry["evidence"].values():
                scopes[item["scope"]] += 1
    print(
        "verification ok: "
        f"{len(manifest['indicators'])} indicators, {len(manifest['events'])} events; "
        f"row={scopes['row']} family={scopes['family']} tool={scopes['tool']} n/a={scopes['n/a']}; "
        f"required={'/'.join(REQUIRED_CLASSES)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

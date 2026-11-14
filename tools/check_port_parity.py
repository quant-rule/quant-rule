#!/usr/bin/env python3
"""Check that every ported computation carries parity evidence.

A port is a computation moved from a retired implementation into Rust. The
policy in ``AGENTS.md`` section 4 says a port is not complete until its numbers
match the retired implementation to 1e-9. This checker enforces the registration
half of that rule: ``docs/port-parity/registry.json`` names each port, its Rust
target, its comparison test, and its golden fixture.

The checker reads only repository files. It does not run the comparison; the
Rust test named by the registry owns that. An empty registry passes, because no
port means no parity claim.
"""

from __future__ import annotations

import json
import math
import re
import sys
from datetime import date
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = "docs/port-parity/registry.json"
REGISTRY_SCHEMA = "quantrule.port-parity/v1"
FIXTURE_SCHEMA = "quantrule.port-parity-fixture/v1"
FIXTURE_DIRECTORY = "docs/port-parity"
TOLERANCE = 1e-9
ID = re.compile(r"^PORT-[0-9]{3}$")
STATUSES = {"active", "retired"}


def load_json(path: Path) -> tuple[object | None, str | None]:
    """Read JSON, returning the value or an error sentence."""
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except OSError as error:
        return None, f"cannot read {path.name}: {error}"
    except json.JSONDecodeError as error:
        return None, f"{path.name} is not valid JSON: {error}"


def relative(value: object, field: str) -> tuple[PurePosixPath | None, str | None]:
    """Validate a repository-relative POSIX path field."""
    if not isinstance(value, str) or not value:
        return None, f"{field} must be a non-empty string"
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts:
        return None, f"{field} must stay inside the repository: {value!r}"
    return path, None


def fixture_problems(root: Path, entry: dict, entry_id: str) -> list[str]:
    """Validate one fixture against its registry entry."""
    expected = f"{FIXTURE_DIRECTORY}/{entry_id}.golden.json"
    if entry["fixture"] != expected:
        return [f"{entry_id}: fixture must be {expected}, found {entry['fixture']}"]

    path = root / expected
    if not path.is_file():
        return [f"{entry_id}: fixture {expected} is missing"]

    value, error = load_json(path)
    if error:
        return [f"{entry_id}: {error}"]
    if not isinstance(value, dict):
        return [f"{entry_id}: fixture must be a JSON object"]

    problems: list[str] = []
    if value.get("schema") != FIXTURE_SCHEMA:
        problems.append(f"{entry_id}: fixture schema must be {FIXTURE_SCHEMA}")
    if value.get("id") != entry_id:
        problems.append(f"{entry_id}: fixture id must match the registry entry")
    if value.get("source") != entry["source"]:
        problems.append(f"{entry_id}: fixture source must match the registry source")

    captured = value.get("captured")
    if not isinstance(captured, str) or not re.fullmatch(r"\d{4}-\d{2}-\d{2}", captured):
        problems.append(f"{entry_id}: fixture captured must be a UTC YYYY-MM-DD date")
    else:
        try:
            if date.fromisoformat(captured).isoformat() != captured:
                problems.append(f"{entry_id}: fixture captured must be a valid UTC date")
        except ValueError:
            problems.append(f"{entry_id}: fixture captured must be a valid UTC date")

    tolerance = value.get("tolerance")
    if (
        not isinstance(tolerance, (int, float))
        or isinstance(tolerance, bool)
        or not math.isfinite(tolerance)
        or tolerance <= 0
    ):
        problems.append(f"{entry_id}: fixture tolerance must be a positive number")
    elif tolerance > TOLERANCE:
        problems.append(
            f"{entry_id}: fixture tolerance {tolerance} is looser than {TOLERANCE}"
        )

    cases = value.get("cases")
    if not isinstance(cases, list) or not cases:
        problems.append(f"{entry_id}: fixture cases must be a non-empty list")
    else:
        for index, case in enumerate(cases):
            if not isinstance(case, dict):
                problems.append(f"{entry_id}: case {index} must be an object")
                continue
            name = case.get("name")
            if not isinstance(name, str) or not name:
                problems.append(f"{entry_id}: case {index} needs a non-empty name")
            for field in ("input", "expected"):
                if field not in case:
                    problems.append(f"{entry_id}: case {index} needs {field}")
    return problems


def entry_problems(root: Path, entry: object, seen: set[str]) -> list[str]:
    """Validate one registry entry and every file it names."""
    if not isinstance(entry, dict):
        return ["registry entries must be objects"]

    required = {"id", "source", "target", "test", "fixture", "status"}
    optional = {"notes"}
    missing = required - entry.keys()
    unknown = entry.keys() - required - optional
    if missing:
        return [f"registry entry is missing {', '.join(sorted(missing))}"]
    if unknown:
        return [f"registry entry has unknown fields: {', '.join(sorted(unknown))}"]

    entry_id = entry["id"]
    if not isinstance(entry_id, str) or not ID.match(entry_id):
        return [f"registry entry id must match PORT-NNN: {entry_id!r}"]
    if entry_id in seen:
        return [f"{entry_id}: duplicate registry entry"]
    seen.add(entry_id)

    problems: list[str] = []
    if not isinstance(entry["source"], str) or not entry["source"]:
        problems.append(f"{entry_id}: source must be a non-empty string")
    status = entry["status"]
    if not isinstance(status, str) or status not in STATUSES:
        problems.append(f"{entry_id}: status must be one of {sorted(STATUSES)}")
    if "notes" in entry and not isinstance(entry["notes"], str):
        problems.append(f"{entry_id}: notes must be a string")
    fixture = entry["fixture"]
    if not isinstance(fixture, str) or not fixture:
        problems.append(f"{entry_id}: fixture must be a non-empty string")

    target, error = relative(entry["target"], "target")
    if error:
        problems.append(f"{entry_id}: {error}")
    elif target.suffix != ".rs":
        problems.append(f"{entry_id}: target must be a Rust source file")
    elif not (root / target).is_file():
        problems.append(f"{entry_id}: target {target} is missing")
    else:
        text = (root / target).read_text(encoding="utf-8", errors="ignore")
        if f"port-parity: {entry_id}" not in text:
            problems.append(f"{entry_id}: target needs the marker port-parity: {entry_id}")

    test, error = relative(entry["test"], "test")
    if error:
        problems.append(f"{entry_id}: {error}")
    elif test.suffix != ".rs":
        problems.append(f"{entry_id}: test must be a Rust source file")
    elif not (root / test).is_file():
        problems.append(f"{entry_id}: test {test} is missing")
    else:
        text = (root / test).read_text(encoding="utf-8", errors="ignore")
        if f"port-parity test: {entry_id}" not in text:
            problems.append(
                f"{entry_id}: test needs the marker port-parity test: {entry_id}"
            )
        if isinstance(fixture, str) and fixture not in text:
            problems.append(
                f"{entry_id}: test {test} must name {fixture}"
            )

    problems.extend(fixture_problems(root, entry, entry_id))
    return problems


def check(root: Path) -> list[str]:
    """Return the port-parity problems under ``root``."""
    path = root / REGISTRY
    if not path.is_file():
        return [f"{REGISTRY} is missing"]

    value, error = load_json(path)
    if error:
        return [error]
    if not isinstance(value, dict):
        return [f"{REGISTRY} must be a JSON object"]
    if value.get("schema") != REGISTRY_SCHEMA:
        return [f"{REGISTRY} schema must be {REGISTRY_SCHEMA}"]
    if set(value) != {"schema", "ports"}:
        return [f"{REGISTRY} must contain exactly schema and ports"]
    ports = value.get("ports")
    if not isinstance(ports, list):
        return [f"{REGISTRY} ports must be a list"]

    problems: list[str] = []
    seen: set[str] = set()
    for entry in ports:
        problems.extend(entry_problems(root, entry, seen))
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    registry = json.loads((ROOT / REGISTRY).read_text(encoding="utf-8"))
    print(f"port parity ok: {len(registry['ports'])} registered port(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

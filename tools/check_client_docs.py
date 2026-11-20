#!/usr/bin/env python3
"""Check that the client document and the model index match the tree.

AUD-2026-11-16-17 found three drifts: the status index named 46 indicators and
143 events while the registry held 49 and 150, the client document described a
Memory screen and a Settings screen that no component implements, and the store
table named files that no longer exist.

This checker reads the screen and store tables in
``docs/architecture/client.md``, the navigation labels in
``apps/web/src/lib/capabilities.ts``, and the counts in ``docs/model/README.md``. Every
component and store a row names must exist, every navigation label must have a
screen row, and the matrix counts must equal the registry. It reads no network,
clock, or external data.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCREEN_HEADER = "| Screen | Component | Shows | Empty state |"
STORE_HEADER = "| Store | Holds | Polling |"
COUNT = {
    "indicator": re.compile(r"\| Indicator verification matrix \| Current; (\d+) rows"),
    "event": re.compile(r"\| Event verification matrix \| Current; (\d+) rows"),
}
SECTIONS_START = "export const SECTIONS = ["
SECTIONS_END = "] as const;"
LABEL = re.compile(r'label:\s*"([^"]+)"')
COMMANDS_START = "export const COMMANDS"
COMMANDS_END = "];"
CAP_FIELD = re.compile(
    r'(command|section|control|component|testId):\s*"([^"]*)"'
)


def table_rows(text: str, header: str) -> list[list[str]] | None:
    """Read the pipe-table rows below ``header``; ``None`` when it is absent."""
    start = text.find(header)
    if start == -1:
        return None
    end = text.find("\n\n", start)
    block = text[start:end if end != -1 else len(text)].splitlines()
    rows: list[list[str]] = []
    for line in block[2:]:
        if not line.strip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells:
            rows.append(cells)
    return rows


def capability_labels(text: str) -> list[str] | None:
    start = text.find(SECTIONS_START)
    end = text.find(SECTIONS_END, start + len(SECTIONS_START)) if start != -1 else -1
    if start == -1 or end == -1:
        return None
    return LABEL.findall(text[start:end])


def capability_entries(text: str) -> list[dict[str, str]]:
    """Return the COMMANDS entries keyed by their declared fields."""
    start = text.find(COMMANDS_START)
    end = text.find(COMMANDS_END, start + len(COMMANDS_START)) if start != -1 else -1
    if start == -1 or end == -1:
        return []
    entries: list[dict[str, str]] = []
    for line in text[start:end].splitlines():
        if "command:" not in line:
            continue
        fields = dict(CAP_FIELD.findall(line))
        if fields:
            entries.append(fields)
    return entries


def unquote(cell: str) -> str:
    return cell.strip().strip("`")


def check(root: Path) -> list[str]:
    problems: list[str] = []
    client_path = root / "docs" / "architecture" / "client.md"
    client = client_path.read_text(encoding="utf-8") if client_path.is_file() else ""
    screens = table_rows(client, SCREEN_HEADER)
    if screens is None:
        problems.append(f"{client_path}: no '{SCREEN_HEADER}' table")
        screens = []

    library = root / "apps" / "web" / "src" / "lib"
    screen_names: list[str] = []
    for row in screens:
        if len(row) < 2:
            problems.append(f"{client_path}: a screen row has fewer than two columns")
            continue
        screen = row[0]
        component = unquote(row[1])
        screen_names.append(screen)
        path = root / component if component.startswith("apps/") else library / component
        if not component or not path.is_file():
            problems.append(
                f"{client_path}: screen {screen!r} names missing component {component!r}"
            )

    stores = table_rows(client, STORE_HEADER)
    if stores is None:
        problems.append(f"{client_path}: no '{STORE_HEADER}' table")
        stores = []
    for row in stores:
        if not row:
            continue
        store = unquote(row[0])
        if store and not (library / store).is_file():
            problems.append(f"{client_path}: store {store!r} does not exist")

    capabilities_path = library / "capabilities.ts"
    if not capabilities_path.is_file():
        problems.append(f"{capabilities_path}: the browser command map is missing")
    else:
        capabilities_text = capabilities_path.read_text(encoding="utf-8")
        labels = capability_labels(capabilities_text)
        if labels is None:
            problems.append(f"{capabilities_path}: the SECTIONS table is missing")
        else:
            for label in labels:
                if label not in screen_names:
                    problems.append(
                        f"{client_path}: navigation label {label!r} has no screen row"
                    )
        entries = capability_entries(capabilities_text)
        if not entries:
            problems.append(f"{capabilities_path}: the COMMANDS table is missing")
        for entry in entries:
            missing = [
                field
                for field in ("command", "section", "control", "component", "testId")
                if not entry.get(field)
            ]
            if missing:
                problems.append(
                    f"{capabilities_path}: {entry.get('command', '?')} is missing "
                    + ", ".join(missing)
                )
                continue
            component = entry["component"]
            path = library / component
            if not path.is_file():
                problems.append(
                    f"{capabilities_path}: {entry['command']} names missing component {component}"
                )
                continue
            test_id = entry["testId"]
            if f'data-testid="{test_id}"' not in path.read_text(encoding="utf-8"):
                problems.append(
                    f"{path}: {entry['command']} control {test_id!r} has no data-testid"
                )

    registry_path = root / "docs" / "registry" / "indicators.json"
    readme_path = root / "docs" / "model" / "README.md"
    if not registry_path.is_file():
        problems.append(f"{registry_path}: the registry is missing")
        return problems
    if not readme_path.is_file():
        problems.append(f"{readme_path}: the model index is missing")
        return problems
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    counts = {
        "indicator": len(registry.get("indicators", [])),
        "event": len(registry.get("events", [])),
    }
    readme = readme_path.read_text(encoding="utf-8")
    for kind, count in counts.items():
        match = COUNT[kind].search(readme)
        if match is None:
            problems.append(f"{readme_path}: the {kind} verification row is missing")
        elif int(match.group(1)) != count:
            problems.append(
                f"{readme_path}: {match.group(1)} {kind} rows, registry has {count}"
            )
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    registry = json.loads((ROOT / "docs/registry/indicators.json").read_text(encoding="utf-8"))
    print(
        "client docs ok: "
        f"{len(registry.get('indicators', []))} indicators, "
        f"{len(registry.get('events', []))} events, "
        "screens and stores resolve"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Check the workflow index, document status, and required sections.

The source of truth is ``docs/workflows/README.md``. Every workflow document must
carry a status line, a purpose section, and a related-documents section, and must
be listed in both the workflow index and the global documentation index.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATUS = re.compile(r"^- Status:\s*(.+)$", re.MULTILINE)
HEADING = re.compile(r"^#{1,6}\s+(.+)$", re.MULTILINE)
ALLOWED_STATUS = (
    "Current",
    "Target",
    "Historical",
    "Deprecated",
    "Proposed",
    "Superseded",
)


def read(path: Path, root: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise ValueError(f"cannot read {path.relative_to(root)}: {error}") from None


def status_of(text: str) -> str | None:
    match = STATUS.search(text)
    if match is None:
        return None
    return match.group(1).strip()


def has_heading(text: str, needle: str) -> bool:
    return any(needle.lower() in heading.lower() for heading in HEADING.findall(text))


def check(root: Path) -> list[str]:
    workflows = root / "docs" / "workflows"
    index = workflows / "README.md"
    global_index = root / "docs" / "README.md"
    templates = workflows / "templates"
    problems: list[str] = []

    if not workflows.is_dir():
        return ["docs/workflows is missing"]
    if not index.is_file():
        problems.append("docs/workflows/README.md is missing")
    if not global_index.is_file():
        problems.append("docs/README.md is missing")

    index_text = read(index, root) if index.is_file() else ""
    global_text = read(global_index, root) if global_index.is_file() else ""

    files = sorted(path for path in workflows.glob("*.md") if path.name != "README.md")
    if not files:
        problems.append("no workflow documents found")

    for path in files:
        text = read(path, root)
        name = path.name
        status = status_of(text)
        if status is None:
            problems.append(f"{name}: missing '- Status:' line")
        elif not status.startswith(ALLOWED_STATUS):
            problems.append(
                f"{name}: status {status!r} must start with one of {', '.join(ALLOWED_STATUS)}"
            )
        if not has_heading(text, "Purpose"):
            problems.append(f"{name}: missing a Purpose heading")
        if not has_heading(text, "Related documents"):
            problems.append(f"{name}: missing a Related documents heading")
        if name not in index_text:
            problems.append(f"{name}: not listed in docs/workflows/README.md")
        if name not in global_text:
            problems.append(f"{name}: not listed in docs/README.md")

    if templates.is_dir():
        for path in sorted(templates.glob("*.md")):
            text = read(path, root)
            if not text.lstrip().startswith("# "):
                problems.append(f"templates/{path.name}: must start with a level-1 heading")

    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    workflows = ROOT / "docs" / "workflows"
    templates = workflows / "templates"
    count = len([path for path in workflows.glob("*.md") if path.name != "README.md"])
    template_count = len(list(templates.glob("*.md"))) if templates.is_dir() else 0
    print(
        "workflow ok: "
        f"{count} workflows, {template_count} templates, index rows {count}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

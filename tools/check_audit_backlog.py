#!/usr/bin/env python3
"""Fail when an audit document leaves a Critical or Major finding Open.

The Audit Workflow's green criteria require every Critical and Major finding to
be Verified, Stale, Duplicate, or human-decided. An Open row is a backlog item
that a release cannot treat as closed.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OPEN_ROW = re.compile(
    r"^\|\s*(AUD-[^|]+?)\s*\|\s*(Critical|Major)\s*\|.*\|\s*Open\s*\|",
    re.MULTILINE,
)


def check(root: Path) -> list[str]:
    directory = root / "docs" / "audits"
    if not directory.is_dir():
        return []
    problems: list[str] = []
    for path in sorted(directory.glob("*/*.md")):
        text = path.read_text(encoding="utf-8", errors="ignore")
        for match in OPEN_ROW.finditer(text):
            problems.append(
                f"{path.relative_to(root).as_posix()}: {match.group(1)} "
                f"({match.group(2)}) is Open"
            )
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print("audit backlog ok: no Critical or Major finding is Open")
    return 0


if __name__ == "__main__":
    sys.exit(main())

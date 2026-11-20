#!/usr/bin/env python3
"""Require the latest release note to name a verified audit status.

A release note must carry ``- Audit status:`` with an audit ID and one of the
closed states. A note that names ``Critical`` or ``Major`` as ``Fixed`` or
``Open`` also needs a recorded residual-risk acceptance. This is the release
gate that catches a release cut before the audit closure.

The checker reads only the repository and no network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIT_STATUS = re.compile(r"^- Audit status:\s*(.+)$", re.MULTILINE)
AUDIT_ID = re.compile(r"AUDIT-\d{4}-\d{2}-\d{2}-[a-z0-9-]+")
SEVERITY = re.compile(r"\b(Critical|Major)\b", re.IGNORECASE)
OPEN_STATE = re.compile(r"\b(Fixed|Open)\b", re.IGNORECASE)
CLOSED_STATE = re.compile(r"\b(Verified|Won't fix|Residual risk accepted)\b", re.IGNORECASE)
RESIDUAL = re.compile(r"Residual risk accepted", re.IGNORECASE)


def latest_note(root: Path) -> Path | None:
    notes = sorted(root.glob("docs/releases/*/v*.md"))
    return notes[-1] if notes else None


def check(root: Path) -> list[str]:
    note = latest_note(root)
    if note is None:
        return ["no release note is present"]
    relative = note.relative_to(root).as_posix()
    text = note.read_text(encoding="utf-8")
    statuses = AUDIT_STATUS.findall(text)
    if not statuses:
        return [f"{relative}: missing '- Audit status:'"]
    problems: list[str] = []
    for status in statuses:
        if AUDIT_ID.search(status) is None:
            problems.append(f"{relative}: the audit status names no audit ID")
        if CLOSED_STATE.search(status) is None:
            problems.append(f"{relative}: the audit status is not Verified, Won't fix, or an accepted risk")
        if SEVERITY.search(status) and OPEN_STATE.search(status) and not RESIDUAL.search(text):
            problems.append(
                f"{relative}: a Critical/Major finding is Fixed or Open without a residual-risk acceptance"
            )
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    note = latest_note(ROOT)
    print(f"release audit ok: {note.relative_to(ROOT).as_posix() if note else 'none'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

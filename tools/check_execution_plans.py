#!/usr/bin/env python3
"""Check the structure of current execution plans.

A plan dated 2026-11-11 or later must carry a status, a claim type, and a
verification table with the reviewer columns. Historical plans are exempt
because they predate the verification-table rule.
"""

from __future__ import annotations

import re
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = date(2026, 11, 11)
EXEMPT = {
    # Recorded before the verification-table and four-claim-type rule.
    "docs/execution-plans/2026-11-11/simulation-data-recovery.md",
}
STATUS = re.compile(r"^- Status:\s*(.+)$", re.MULTILINE)
CLAIM = re.compile(r"^- Claim type:\s*(.+)$", re.MULTILINE)
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
CLAIM_TYPES = (
    "Problem Claim",
    "Behaviour Claim",
    "Outcome Claim",
    "ADR (Architecture Decision)",
)
COLUMNS = (
    "id",
    "plan item",
    "acceptance criterion",
    "evidence / command",
    "expected",
    "observed",
    "status",
    "reviewer",
    "date",
    "notes",
)


def table_header(text: str) -> str | None:
    for line in text.splitlines():
        if line.lower().startswith("| id |") and "acceptance criterion" in line.lower():
            return line
    return None


def check(root: Path) -> list[str]:
    plans = root / "docs" / "execution-plans"
    if not plans.is_dir():
        return ["docs/execution-plans is missing"]
    problems: list[str] = []
    for path in sorted(plans.glob("*/*.md")):
        directory = path.parent.name
        if not DATE.match(directory):
            problems.append(f"{path.relative_to(root)}: plan directory must be YYYY-MM-DD")
            continue
        if date.fromisoformat(directory) < BASELINE:
            continue
        relative = path.relative_to(root).as_posix()
        if relative in EXEMPT:
            continue
        text = path.read_text(encoding="utf-8")
        status = STATUS.search(text)
        if status is None:
            problems.append(f"{relative}: missing '- Status:' line")
        claim = CLAIM.search(text)
        if claim is None:
            problems.append(f"{relative}: missing '- Claim type:' line")
        elif not any(name in claim.group(1) for name in CLAIM_TYPES):
            problems.append(f"{relative}: claim type is not one of the four approved values")
        marker = "## Verification table"
        if marker not in text:
            problems.append(f"{relative}: missing '## Verification table' section")
        else:
            section = text.split(marker, 1)[1].split("\n## ", 1)[0]
            header = table_header(section)
            if header is None:
                problems.append(f"{relative}: verification table header is missing or incomplete")
            else:
                lower = header.lower()
                missing = [column for column in COLUMNS if column not in lower]
                if missing:
                    problems.append(
                        f"{relative}: verification table is missing columns {', '.join(missing)}"
                    )
                rows = [
                    line
                    for line in section.splitlines()
                    if line.startswith("|") and not set(line) <= {"|", "-", " ", ":"}
                ]
                if len(rows) <= 1:
                    problems.append(f"{relative}: verification table has no data rows")
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print("execution plans ok: current plans carry a claim type and verification table")
    return 0


if __name__ == "__main__":
    sys.exit(main())

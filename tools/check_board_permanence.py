#!/usr/bin/env python3
"""Check the project board permanence rule and the board automation.

The board is append-only project history. The normative rule must be present in
``AGENTS.md``, and no board script or workflow may call a delete or archive
mutation. The checker reads only repository files; it never reaches GitHub.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RULE = "The board is append-only history."
BOARD_SCRIPTS = (".github/scripts/project-sync.sh",)
BOARD_WORKFLOWS = ".github/workflows"
FORBIDDEN = (
    "deleteProjectV2Item",
    "archiveProjectV2Item",
    "gh project item-delete",
    "gh project item-archive",
)


def check(root: Path) -> list[str]:
    """Return the board-permanence problems under ``root``."""
    problems: list[str] = []

    agents = root / "AGENTS.md"
    if not agents.is_file():
        problems.append("AGENTS.md is missing")
    elif RULE not in agents.read_text(encoding="utf-8"):
        problems.append(f"AGENTS.md is missing the board permanence rule: {RULE!r}")

    paths = [root / relative for relative in BOARD_SCRIPTS]
    workflows = root / BOARD_WORKFLOWS
    if workflows.is_dir():
        paths.extend(sorted(workflows.glob("*.yml")))

    for path in paths:
        if not path.is_file():
            problems.append(f"{path.relative_to(root)} is missing")
            continue
        text = path.read_text(encoding="utf-8")
        for forbidden in FORBIDDEN:
            if forbidden in text:
                problems.append(
                    f"{path.relative_to(root)} contains forbidden board mutation "
                    f"{forbidden!r}"
                )

    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print("board permanence ok: append-only rule present; no delete or archive mutation")
    return 0


if __name__ == "__main__":
    sys.exit(main())

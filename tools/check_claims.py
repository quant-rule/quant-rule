#!/usr/bin/env python3
"""Check that issue and pull-request templates carry the claim type.

Live GitHub issues cannot be checked from the repository, so this checker
protects the surfaces that are checked in: every issue template, the pull
request template, and the normative claim-type section in AGENTS.md.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLAIM_TYPES = (
    "Problem Claim",
    "Behaviour Claim",
    "Outcome Claim",
    "ADR (Architecture Decision)",
)


def check(root: Path) -> list[str]:
    problems: list[str] = []
    templates = root / ".github" / "ISSUE_TEMPLATE"
    if not templates.is_dir():
        problems.append(".github/ISSUE_TEMPLATE is missing")
    else:
        for path in sorted(templates.glob("*.md")):
            text = path.read_text(encoding="utf-8")
            if "Claim type:" not in text:
                problems.append(f"{path.relative_to(root)}: missing 'Claim type:' line")
            elif not all(name in text for name in CLAIM_TYPES):
                problems.append(
                    f"{path.relative_to(root)}: claim-type line does not name all four types"
                )
    pull_request = root / ".github" / "pull_request_template.md"
    if not pull_request.is_file():
        problems.append(".github/pull_request_template.md is missing")
    else:
        text = pull_request.read_text(encoding="utf-8")
        if "Claim type:" not in text:
            problems.append("pull_request_template.md: missing 'Claim type:' line")
        elif not all(name in text for name in CLAIM_TYPES):
            problems.append("pull_request_template.md: claim-type line does not name all four types")
    agents = root / "AGENTS.md"
    if not agents.is_file():
        problems.append("AGENTS.md is missing")
    else:
        text = agents.read_text(encoding="utf-8")
        if "### Claim type" not in text:
            problems.append("AGENTS.md: missing '### Claim type' section")
        for name in CLAIM_TYPES:
            if name not in text:
                problems.append(f"AGENTS.md: missing claim type {name!r}")
    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print("claim types ok: templates and AGENTS.md carry the four approved types")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Fail when a Rust source file exceeds the project's 1,000-line rule.

The rule is in ``AGENTS.md`` section 3. The checker scans every ``*.rs`` file
under ``crates/`` and counts physical lines. It has no dependency beyond the
Python standard library.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
LIMIT = 1_000


def lines_in(path: Path) -> int:
    text = path.read_text(encoding="utf-8", errors="ignore")
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def main() -> int:
    violations = []
    for path in sorted(CRATES.rglob("*.rs")):
        count = lines_in(path)
        if count > LIMIT:
            violations.append((path.relative_to(ROOT).as_posix(), count))
    if violations:
        for name, count in violations:
            print(f"error: {name} has {count} lines; the limit is {LIMIT}", file=sys.stderr)
        return 1
    print(f"file size ok: every Rust source file is at or under {LIMIT} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())

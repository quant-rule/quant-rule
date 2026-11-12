#!/usr/bin/env python3
"""Check the length caps for current living documents.

Caps exist so a living document stays a page a reader can act on. They apply to
documents that are current by definition: the architecture set, and the records
created from 2026-11-12 onward. Historical records are never rewritten to fit a
cap.

Usage:
    python3 tools/check_doc_length.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ARCHITECTURE = ROOT / "docs/architecture"

# Per-file overrides for reference documents that are large by nature.
OVERRIDES = {
    "command-surface.md": 330,
    "http-api.md": 210,
}
DEFAULT_ARCHITECTURE = 170
ADR_CAP = 110
TUTORIAL_CAP = 200
CURRENT_FROM = "2026-11-12"

DATE_DIR = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def current_since(path: Path) -> bool:
    """Return whether a dated work-item path is current."""
    for part in path.relative_to(ROOT).parts:
        if DATE_DIR.match(part):
            return part >= CURRENT_FROM
    return False


def targets() -> list[tuple[Path, int]]:
    found: list[tuple[Path, int]] = []
    if ARCHITECTURE.is_dir():
        for path in sorted(ARCHITECTURE.glob("*.md")):
            found.append((path, OVERRIDES.get(path.name, DEFAULT_ARCHITECTURE)))
    for kind, cap in (("adr", ADR_CAP), ("tutorials", TUTORIAL_CAP)):
        base = ROOT / "docs" / kind
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.md")):
            if current_since(path):
                found.append((path, cap))
    return found


def main() -> int:
    errors: list[str] = []
    for path, cap in targets():
        try:
            lines = len(path.read_text(encoding="utf-8").splitlines())
        except OSError as error:
            errors.append(f"{path.relative_to(ROOT)}: cannot read: {error}")
            continue
        if lines > cap:
            errors.append(
                f"{path.relative_to(ROOT)}: {lines} lines, cap {cap}"
            )
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        print(f"document length failed: {len(errors)} file(s) over cap", file=sys.stderr)
        return 1
    print(f"document length ok: {len(targets())} file(s) within cap")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

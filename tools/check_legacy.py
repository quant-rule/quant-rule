#!/usr/bin/env python3
"""Fail when a retired model or strategy label appears in a current artifact.

History directories are not scanned: ADRs, audits, improvement records, AI
transparency logs, execution plans, releases, and reports keep their original
names. The naming registry is the one current document that must name the old
aliases; it is allowlisted explicitly.

Paths that P3/P4/P6 still remove are listed as pending. The list shrinks with
each phase and is empty at P7.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SCAN = [
    ROOT / "README.md",
    ROOT / "docs/README.md",
    ROOT / "docs/architecture",
    ROOT / "docs/model/2026-11-05",
    ROOT / "docs/config",
    ROOT / "config/defaults",
    ROOT / "config/examples",
    ROOT / "config/research",
]

# The alias registry must name the old labels to map them.
ALLOWLIST = {
    "docs/model/2026-11-05/model-families.md",
}

# Pending current-surface paths that P3, P4, or P6 remove.
PENDING = (
    "docs/config/2026-10-",
    "docs/config/2026-11-01",
    "docs/tutorials/",
    "docs/model/2026-10-",
    "docs/model/2026-11-01",
    "docs/model/2026-11-03/model-v3.md",
    "config/examples/",
    "config/research/",
)

TOKENS = [
    "model v3",
    "model v4",
    "quantrule.model/v1",
    "quantrule.model/v2",
    "quantrule.model/v3",
    "strategy v2",
    "strategy v3",
    "strategy v4",
    "strategy v5",
    "strategy v6",
    "strategy v7",
    "strategy v8",
    "strategy v9",
    "strategy v10",
    "quantrule.strategy/v2",
    "quantrule.strategy/v3",
    "quantrule.strategy/v4",
    "quantrule.strategy/v5",
    "quantrule.strategy/v6",
    "quantrule.strategy/v7",
    "quantrule.strategy/v8",
    "v5 programme",
]
PATTERN = re.compile("|".join(re.escape(token) for token in TOKENS), re.IGNORECASE)


def files() -> list[Path]:
    found: list[Path] = []
    for entry in SCAN:
        if entry.is_file():
            found.append(entry)
        elif entry.is_dir():
            found.extend(path for path in entry.rglob("*") if path.is_file())
    return sorted(set(found))


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def main() -> int:
    errors: list[str] = []
    for path in files():
        name = relative(path)
        if name in ALLOWLIST or name.startswith(PENDING):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        for number, line in enumerate(lines, 1):
            match = PATTERN.search(line)
            if match:
                errors.append(f"{name}:{number}: retired label {match.group(0)!r}")
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"legacy scan ok: {len(files())} current files")
    return 0


if __name__ == "__main__":
    sys.exit(main())

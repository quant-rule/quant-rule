#!/usr/bin/env python3
"""Run the repository term gate over the active documentation.

This is a thin wrapper around ``tools/improvement/cleanup.py``. It scans docs,
the root README, and AGENTS.md, while excluding the AI transparency log and the
invalidation tombstone: both record retired terms on purpose.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

CLEANUP = Path(__file__).resolve().parent / "improvement" / "cleanup.py"
ARGS = [
    "--check",
    "--paths",
    "docs",
    "README.md",
    "AGENTS.md",
    "--exclude",
    "docs/ai-transparency-log",
    "docs/improvement/INVALIDATIONS.md",
    "--terms-only",
]


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="check_terms.py",
        description=(
            "Run the repository term gate over docs, README.md, and AGENTS.md. "
            "The AI transparency log and the invalidation tombstone are excluded "
            "because they record retired terms on purpose."
        ),
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    parse_arguments(argv)
    return subprocess.call([sys.executable, str(CLEANUP), *ARGS])


if __name__ == "__main__":
    sys.exit(main())

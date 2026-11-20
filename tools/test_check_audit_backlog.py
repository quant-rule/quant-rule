#!/usr/bin/env python3
"""Unit tests for the audit backlog gate."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_audit_backlog import check


def audit(root: Path, status: str) -> None:
    path = root / "docs" / "audits" / "2026-01-01" / "sample-audit.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "# Sample\n\n| ID | Severity | Area | Finding | Status | Links |\n"
        f"|---|---|---|---|---|---|\n| AUD-2026-01-01-01 | Major | test | finding | {status} | — |\n",
        encoding="utf-8",
    )


class AuditBacklogTests(unittest.TestCase):
    def test_an_open_major_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            audit(root, "Open")
            self.assertTrue(any("is Open" in problem for problem in check(root)))

    def test_a_fixed_major_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            audit(root, "Fixed")
            self.assertEqual(check(root), [])


if __name__ == "__main__":
    unittest.main()

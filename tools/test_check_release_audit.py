#!/usr/bin/env python3
"""Unit tests for the release-note audit gate."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_release_audit import check


def note(root: Path, body: str) -> None:
    path = root / "docs" / "releases" / "2026-01-01" / "v1.0.0.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


class ReleaseAuditTests(unittest.TestCase):
    def test_a_verified_status_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            note(root, "- Audit status: AUDIT-2026-01-01-test Verified 2026-01-02\n")
            self.assertEqual(check(root), [])

    def test_a_fixed_critical_without_acceptance_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            note(root, "- Audit status: AUDIT-2026-01-01-test Fixed Critical\n")
            problems = check(root)
            self.assertTrue(any("residual-risk" in problem for problem in problems), problems)

    def test_a_missing_status_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            note(root, "# Release\n")
            problems = check(root)
            self.assertTrue(any("missing" in problem for problem in problems), problems)


if __name__ == "__main__":
    unittest.main()

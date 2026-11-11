#!/usr/bin/env python3
"""Unit tests for the claim-type checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_claims

CLAIM_LINE = (
    "Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, "
    "or ADR (Architecture Decision)\n"
)


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def make_tree(root: Path, template: str | None = None) -> None:
    write(root / ".github" / "ISSUE_TEMPLATE" / "task.md", template or CLAIM_LINE)
    write(root / ".github" / "pull_request_template.md", CLAIM_LINE)
    write(
        root / "AGENTS.md",
        "### Claim type\n\nProblem Claim\nBehaviour Claim\nOutcome Claim\nADR (Architecture Decision)\n",
    )


class ClaimTypeCheckerTests(unittest.TestCase):
    def test_valid_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root)
            self.assertEqual(check_claims.check(root), [])

    def test_missing_claim_line_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root, "# Task\n")
            problems = check_claims.check(root)
            self.assertTrue(any("missing 'Claim type:'" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()

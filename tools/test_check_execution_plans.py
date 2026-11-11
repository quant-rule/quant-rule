#!/usr/bin/env python3
"""Unit tests for the execution-plan checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_execution_plans


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def plan_doc(table: bool = True) -> str:
    table_text = (
        "\n## Verification table\n\n"
        "| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |\n"
        "|---|---|---|---|---|---|---|---|---|---|\n"
        "| V1 | Example | It passes | `true` | Pass | Pass | Pass | Reviewer | 2026-11-11 | |\n"
    )
    return (
        "# Example Plan\n\n"
        "- Status: Approved\n"
        "- Claim type: Behaviour Claim\n\n"
        "## Intent\n\nExample.\n"
        + (table_text if table else "")
    )


class ExecutionPlanCheckerTests(unittest.TestCase):
    def test_valid_plan_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "execution-plans" / "2026-11-11" / "example.md", plan_doc())
            self.assertEqual(check_execution_plans.check(root), [])

    def test_missing_table_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "execution-plans" / "2026-11-11" / "example.md",
                plan_doc(table=False),
            )
            problems = check_execution_plans.check(root)
            self.assertTrue(any("Verification table" in problem for problem in problems))

    def test_historical_plan_is_exempt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "execution-plans" / "2026-11-10" / "old.md", "# Old\n")
            self.assertEqual(check_execution_plans.check(root), [])


if __name__ == "__main__":
    unittest.main()

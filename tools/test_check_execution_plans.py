#!/usr/bin/env python3
"""Unit tests for the execution-plan checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_execution_plans

PASS_ROW = (
    "| V1 | Example | It passes | `true` | Pass | Pass | Pass | Reviewer "
    "| 2026-11-11 | |"
)


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def plan_doc(
    table: bool = True,
    status: str = "Approved",
    rows: list[str] | None = None,
) -> str:
    table_rows = [PASS_ROW] if rows is None else rows
    table_text = (
        "\n## Verification table\n\n"
        "| ID | Plan item | Acceptance criterion | Evidence / command | Expected "
        "| Observed | Status | Reviewer | Date | Notes |\n"
        "|---|---|---|---|---|---|---|---|---|---|\n"
        + "\n".join(table_rows)
        + "\n"
    )
    return (
        "# Example Plan\n\n"
        f"- Status: {status}\n"
        "- Claim type: Behaviour Claim\n\n"
        "## Intent\n\nExample.\n"
        + (table_text if table else "")
    )


class ExecutionPlanCheckerTests(unittest.TestCase):
    def check(self, text: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "execution-plans" / "2026-11-11" / "example.md", text)
            return check_execution_plans.check(root)

    def test_valid_plan_passes(self) -> None:
        self.assertEqual(self.check(plan_doc()), [])

    def test_missing_table_is_reported(self) -> None:
        problems = self.check(plan_doc(table=False))
        self.assertTrue(any("Verification table" in problem for problem in problems))

    def test_historical_plan_is_exempt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "execution-plans" / "2026-11-10" / "old.md", "# Old\n")
            self.assertEqual(check_execution_plans.check(root), [])

    def test_duplicate_id_is_reported(self) -> None:
        problems = self.check(plan_doc(rows=[PASS_ROW, PASS_ROW]))
        self.assertTrue(any("duplicate" in problem for problem in problems))

    def test_empty_plan_item_is_reported(self) -> None:
        row = (
            "| V1 |  | It passes | `true` | Pass | Pass | Pass | Reviewer "
            "| 2026-11-11 | |"
        )
        problems = self.check(plan_doc(rows=[row]))
        self.assertTrue(any("empty Plan item" in problem for problem in problems))

    def test_verified_plan_with_pending_is_reported(self) -> None:
        row = (
            "| V1 | Example | It passes | `true` | Pass | Pending | Pending | Reviewer "
            "| 2026-11-11 | |"
        )
        problems = self.check(plan_doc(status="Verified", rows=[row]))
        self.assertTrue(any("Verified plan row V1" in problem for problem in problems))

    def test_verified_plan_with_pass_passes(self) -> None:
        self.assertEqual(self.check(plan_doc(status="Verified")), [])

    def test_escaped_pipe_in_evidence_passes(self) -> None:
        row = (
            '| V1 | Example | It passes | `rg "a\\|b"` | Pass | Pass | Pass '
            "| Reviewer | 2026-11-11 | |"
        )
        self.assertEqual(self.check(plan_doc(rows=[row])), [])


if __name__ == "__main__":
    unittest.main()

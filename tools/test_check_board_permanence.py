#!/usr/bin/env python3
"""Unit tests for the board permanence checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_board_permanence


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def tree(root: Path, rule: bool = True, script: str = "", workflow: str = "") -> None:
    write(
        root / "AGENTS.md",
        "# Agents\n\n"
        + (f"{check_board_permanence.RULE}\n" if rule else "")
        + "No deletion.\n",
    )
    write(root / ".github" / "scripts" / "project-sync.sh", script)
    write(root / ".github" / "workflows" / "ci.yml", workflow)


class BoardPermanenceCheckerTests(unittest.TestCase):
    def test_valid_board_automation_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, script="set -euo pipefail\n", workflow="name: CI\n")
            self.assertEqual(check_board_permanence.check(root), [])

    def test_missing_rule_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, rule=False, script="set -euo pipefail\n", workflow="name: CI\n")
            problems = check_board_permanence.check(root)
            self.assertTrue(
                any("board permanence rule" in problem for problem in problems)
            )

    def test_delete_mutation_in_script_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(
                root,
                script="deleteProjectV2Item(input: {})\n",
                workflow="name: CI\n",
            )
            problems = check_board_permanence.check(root)
            self.assertTrue(
                any("deleteProjectV2Item" in problem for problem in problems)
            )

    def test_archive_command_in_workflow_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(
                root,
                script="set -euo pipefail\n",
                workflow="run: gh project item-archive 1 --owner quant-rule\n",
            )
            problems = check_board_permanence.check(root)
            self.assertTrue(
                any("item-archive" in problem for problem in problems)
            )


if __name__ == "__main__":
    unittest.main()

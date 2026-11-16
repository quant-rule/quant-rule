#!/usr/bin/env python3
"""Unit tests for the workflow index checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_workflows


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def workflow_doc(status: str = "Current") -> str:
    return (
        "# Example Workflow\n\n"
        f"- Status: {status}\n\n"
        "## Purpose\n\nExample.\n\n"
        "## Related documents\n\n- [Index](README.md)\n"
    )


def make_tree(root: Path, workflow: str | None = None) -> None:
    write(root / "docs" / "README.md", "# Docs\n\n[workflow](workflows/example.md)\n")
    write(
        root / "docs" / "workflows" / "README.md",
        "# Workflows\n\n[example](example.md)\n",
    )
    write(
        root / "docs" / "workflows" / "example.md",
        workflow if workflow is not None else workflow_doc(),
    )
    write(
        root / "docs" / "workflows" / "templates" / "workflow-template.md",
        "# Template\n\n- Status: Current\n",
    )


class WorkflowCheckerTests(unittest.TestCase):
    def test_valid_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root)
            self.assertEqual(check_workflows.check(root), [])

    def test_missing_status_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root, "# Example\n\n## Purpose\n\n## Related documents\n")
            problems = check_workflows.check(root)
            self.assertTrue(any("missing '- Status:'" in problem for problem in problems))

    def test_unknown_status_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root, workflow_doc("Draft"))
            problems = check_workflows.check(root)
            self.assertTrue(any("status 'Draft'" in problem for problem in problems))

    def test_missing_purpose_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root, "# Example\n\n- Status: Current\n\n## Related documents\n")
            problems = check_workflows.check(root)
            self.assertTrue(any("Purpose" in problem for problem in problems))

    def test_missing_index_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_tree(root)
            write(root / "docs" / "README.md", "# Docs\n")
            problems = check_workflows.check(root)
            self.assertTrue(any("docs/README.md" in problem for problem in problems))

    def test_a_current_workflow_linking_a_historical_one_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "docs" / "workflows"
            write(root / "current.md", workflow_doc("Current"))
            write(root / "old.md", workflow_doc("Historical"))
            write(
                root / "current.md",
                workflow_doc("Current")
                + "\nUse the [old process](old.md) when the envelope changes.\n",
            )
            problems = check_workflows.historical_links(
                [root / "current.md", root / "old.md"], Path(temporary)
            )
            self.assertEqual(len(problems), 1, problems)
            self.assertIn("old.md", problems[0])

    def test_a_qualified_historical_reference_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "docs" / "workflows"
            write(root / "current.md", workflow_doc("Current"))
            write(root / "old.md", workflow_doc("Historical"))
            write(
                root / "current.md",
                workflow_doc("Current")
                + "\nThe historical [old process](old.md) is retained for context.\n",
            )
            self.assertEqual(
                check_workflows.historical_links(
                    [root / "current.md", root / "old.md"], Path(temporary)
                ),
                [],
            )


if __name__ == "__main__":
    unittest.main()

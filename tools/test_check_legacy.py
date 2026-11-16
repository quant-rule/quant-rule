#!/usr/bin/env python3
"""Unit tests for the legacy-label and retired-path gate.

The DIST-105 tests build a minimal tree so a reintroduced retired tool path
cannot silently stop failing the gate.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_legacy


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class RetiredToolPathTests(unittest.TestCase):
    def test_a_current_document_naming_a_retired_path_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "workflows" / "example.md",
                "Run `python3 tools/simulation/run.py` for the harness.\n",
            )
            errors = check_legacy.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("docs/workflows/example.md:1", errors[0])
            self.assertIn("tools/simulation", errors[0])

    def test_every_retired_path_is_seeded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for index, path in enumerate(check_legacy.RETIRED_PATHS, 1):
                write(root / "notes" / f"{index}.md", f"see {path}/README.md\n")
            errors = check_legacy.check(root)
            found = {
                path
                for path in check_legacy.RETIRED_PATHS
                if any(path in error for error in errors)
            }
            self.assertEqual(found, set(check_legacy.RETIRED_PATHS), errors)

    def test_a_clean_current_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "README.md", "The simulation service renders the chart.\n")
            self.assertEqual(check_legacy.check(root), [])

    def test_history_is_not_scanned(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "audits" / "2026-11-10" / "example-audit.md",
                "The audit ran `python3 tools/oracle/progression_replay.py`.\n",
            )
            write(
                root / "docs" / "ai-transparency-log" / "2026-11-15" / "log.md",
                "The log names tools/charts/annotate.py.\n",
            )
            self.assertEqual(check_legacy.check(root), [])

    def test_the_allowlisted_record_still_names_the_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "registry" / "history.md",
                "| `tools/trades/` | retired |\n",
            )
            self.assertEqual(check_legacy.check(root), [])


class RetiredLabelTests(unittest.TestCase):
    def test_a_retired_label_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "crates" / "example" / "src" / "lib.rs", "// model v3\n")
            errors = check_legacy.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("retired label", errors[0])

    def test_an_allowlisted_negative_test_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-app" / "src" / "config" / "tests.rs",
                "// model v3 is rejected\n",
            )
            self.assertEqual(check_legacy.check(root), [])

    def test_a_retired_label_in_a_current_workflow_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "workflows" / "example.md",
                "The model v3 contract applies here.\n",
            )
            errors = check_legacy.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("docs/workflows/example.md:1", errors[0])
            self.assertIn("retired label", errors[0])

    def test_a_retired_label_in_agents_md_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "AGENTS.md", "The current surface is model v3.\n")
            errors = check_legacy.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("AGENTS.md:1", errors[0])

    def test_a_retired_label_in_history_is_exempt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "audits" / "2026-11-10" / "example.md",
                "The auditor found model v3 in the tree.\n",
            )
            self.assertEqual(check_legacy.check(root), [])


if __name__ == "__main__":
    unittest.main()

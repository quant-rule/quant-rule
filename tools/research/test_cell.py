#!/usr/bin/env python3
"""Unit tests for the research cell runner."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

import cell
import program
from test_program import document, write

FIXTURE = Path(__file__).resolve().parent / "tests" / "fixture_cell.py"


def one_cell(verdict: str, validation_access: str = "none") -> dict:
    value = document()
    value["cells"] = [value["cells"][0]]
    value["correction"]["primary_cells"] = 1
    value["cells"][0]["command"] = [
        sys.executable,
        str(FIXTURE),
        "--cell",
        "CELL-01",
        "--out",
        "{out}",
        "--verdict",
        verdict,
    ]
    value["cells"][0]["result_parser"] = "summary_json"
    value["validation_access"] = validation_access
    return value


class CellTests(unittest.TestCase):
    def test_a_passing_cell_writes_a_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, one_cell("pass"))
            loaded = program.load_program(path, repo_root=root)
            out = root / "out" / "CELL-01"
            record = cell.run_cell(loaded, loaded.cells[0], "dev", out)
            self.assertEqual(record["verdict"], "pass")
            self.assertEqual(record["metrics"]["n"], 100)
            self.assertTrue((out / "cell-result.json").is_file())
            self.assertTrue(
                any(item["path"] == "summary.json" for item in record["artifacts"])
            )

    def test_a_failing_cell_writes_a_fail_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, one_cell("fail"))
            loaded = program.load_program(path, repo_root=root)
            record = cell.run_cell(loaded, loaded.cells[0], "dev", root / "out")
            self.assertEqual(record["verdict"], "fail")

    def test_validation_is_rejected_without_one_shot_access(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, one_cell("pass", validation_access="none"))
            loaded = program.load_program(path, repo_root=root)
            with self.assertRaises(program.ProgramError):
                cell.run_cell(loaded, loaded.cells[0], "validation", root / "out")

    def test_validation_is_allowed_with_one_shot_access(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, one_cell("pass", validation_access="one_shot"))
            loaded = program.load_program(path, repo_root=root)
            record = cell.run_cell(loaded, loaded.cells[0], "validation", root / "out")
            self.assertEqual(record["phase"], "validation")


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the parallel research batch runner."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

import batch
import program
from test_program import cell, document, write

FIXTURE = Path(__file__).resolve().parent / "tests" / "fixture_cell.py"


def three_cells() -> dict:
    cells = [cell("CELL-01"), cell("CELL-02"), cell("CELL-03")]
    for item in cells:
        item["command"] = [
            sys.executable,
            str(FIXTURE),
            "--cell",
            item["cell_id"],
            "--out",
            "{out}",
        ]
        item["result_parser"] = "summary_json"
    value = document(cells)
    value["correction"]["primary_cells"] = 3
    value["max_parallel"] = 3
    return value


class BatchTests(unittest.TestCase):
    def test_three_cells_run_in_config_mode(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cells())
            loaded = program.load_program(path, repo_root=root)
            out = root / "results"
            summary = batch.run_config(loaded, "dev", loaded.cells, 3, out)
            self.assertEqual(len(summary["cells"]), 3)
            self.assertEqual(summary["mode"], "config")
            for item in summary["cells"]:
                self.assertEqual(item["verdict"], "fail")
                self.assertTrue(
                    (out / item["cell_id"] / "cell-result.json").is_file()
                )
            self.assertTrue((out / "batch-summary.json").is_file())

    def test_cell_filter_selects_only_requested_cells(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cells())
            loaded = program.load_program(path, repo_root=root)
            selected = batch.selected_cells(loaded, "CELL-01,CELL-03")
            self.assertEqual([item.cell_id for item in selected], ["CELL-01", "CELL-03"])

    def test_unknown_cell_filter_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cells())
            loaded = program.load_program(path, repo_root=root)
            with self.assertRaises(program.ProgramError):
                batch.selected_cells(loaded, "CELL-99")


if __name__ == "__main__":
    unittest.main()

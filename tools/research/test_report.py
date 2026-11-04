#!/usr/bin/env python3
"""Unit tests for the research program aggregator."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import program
import report
from test_program import cell, document, write


def result(path: Path, verdict: str, lift: float) -> None:
    path.mkdir(parents=True, exist_ok=True)
    document_value = {
        "schema": "quantrule.research-cell-result/v1",
        "program_id": "PROGRAM-2026-11-04-90",
        "cell_id": path.name,
        "phase": "dev",
        "verdict": verdict,
        "metrics": {
            "n": 100,
            "clusters": 20,
            "lift": lift,
            "lift_lcb": lift - 0.01,
            "grade": "B" if verdict == "pass" else "C",
        },
        "limitations": ["fixture"],
    }
    (path / "cell-result.json").write_text(
        json.dumps(document_value), encoding="utf-8"
    )


def three_cell_program() -> dict:
    value = document([cell("CELL-01"), cell("CELL-02"), cell("CELL-03")])
    value["correction"]["primary_cells"] = 3
    return value


class ReportTests(unittest.TestCase):
    def test_all_fail_is_no_edge(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cell_program())
            loaded = program.load_program(path, repo_root=root)
            results = root / "results"
            for cell_id in ("CELL-01", "CELL-02", "CELL-03"):
                result(results / cell_id, "fail", 0.01)
            document_value = report.build(loaded, results)
            self.assertTrue(document_value["decision"].startswith("no_edge"))
            self.assertIsNone(document_value["candidate"])
            self.assertEqual(len(document_value["cells"]), 3)
            self.assertIn("No primary cell passed", report.markdown(document_value))

    def test_one_pass_is_a_candidate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cell_program())
            loaded = program.load_program(path, repo_root=root)
            results = root / "results"
            result(results / "CELL-01", "fail", 0.01)
            result(results / "CELL-02", "pass", 0.06)
            result(results / "CELL-03", "fail", 0.01)
            document_value = report.build(loaded, results)
            self.assertTrue(document_value["decision"].startswith("candidate"))
            self.assertEqual(document_value["candidate"], "CELL-02")
            self.assertIn("Candidate for human review", report.markdown(document_value))

    def test_missing_result_is_marked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cell_program())
            loaded = program.load_program(path, repo_root=root)
            results = root / "results"
            result(results / "CELL-01", "fail", 0.01)
            document_value = report.build(loaded, results)
            self.assertEqual(document_value["cells"][1]["verdict"], "missing")
            self.assertTrue(document_value["decision"].startswith("no_edge"))

    def test_correction_is_recorded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, three_cell_program())
            loaded = program.load_program(path, repo_root=root)
            correction = report.correction(loaded)
            self.assertEqual(correction["primary_cells"], 3)
            self.assertAlmostEqual(correction["cell_alpha"], 0.05 / 3)
            self.assertGreater(correction["z"], 1.96)


if __name__ == "__main__":
    unittest.main()

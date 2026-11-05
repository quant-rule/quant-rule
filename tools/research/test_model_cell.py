#!/usr/bin/env python3
"""Dry-run tests for the model-cell runner."""

from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

import model_cell

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path(__file__).resolve().parent / "tests" / "fixture_model_shell.py"
EVALUATOR = ROOT / "tools" / "improvement" / "model_prediction.py"


def make_cell(root: Path) -> tuple[Path, Path]:
    cell = root / "cells" / "M2.test"
    cell.mkdir(parents=True)
    for instrument in model_cell.INSTRUMENTS:
        (cell / f"spec-{instrument}.json").write_text("{}\n", encoding="utf-8")
        run = root / "runs" / instrument / "run.json"
        run.parent.mkdir(parents=True)
        run.write_text("{}\n", encoding="utf-8")
    qr = root / "fixture_quantrule"
    shutil.copy2(FIXTURE, qr)
    qr.chmod(qr.stat().st_mode | 0o111)
    return cell, qr


def run(cell: Path, qr: Path, out: Path, phase: str = "dev") -> int:
    return model_cell.main(
        [
            "--cell-dir",
            str(cell),
            "--phase",
            phase,
            "--out",
            str(out),
            "--runs-dir",
            str(cell.parent.parent / "runs"),
            "--qr",
            str(qr),
            "--evaluator",
            str(EVALUATOR),
            "--repo-root",
            str(cell.parent.parent),
            "--primary-cells",
            "5",
            "--seed",
            "0",
        ]
    )


class ModelCellTests(unittest.TestCase):
    def test_a_dry_run_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cell, qr = make_cell(root)
            out = root / "out"
            self.assertEqual(run(cell, qr, out), 0)
            first = (out / "model_prediction.json").read_bytes()
            self.assertEqual(run(cell, qr, out), 0)
            second = (out / "model_prediction.json").read_bytes()
            document = json.loads(first)
        self.assertEqual(first, second)
        self.assertEqual(document["phase"], "dev")
        self.assertEqual(document["partition"], "model_dev")
        self.assertEqual(document["spec_id"], "M2.test")
        self.assertEqual(len(document["prediction_digest"]), 64)
        self.assertEqual(
            set(document["metrics"]["instruments"]),
            set(model_cell.INSTRUMENTS),
        )
        self.assertIn(document["grade"], {"A", "B"})
        self.assertEqual(document["verdict"], "pass")

    def test_validation_uses_the_validation_split(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cell, qr = make_cell(root)
            out = root / "out"
            self.assertEqual(run(cell, qr, out, "validation"), 0)
            document = json.loads((out / "model_prediction.json").read_text())
        self.assertEqual(document["phase"], "validation")
        self.assertEqual(document["partition"], "model_validation")
        self.assertEqual(document["metrics"]["n"], 0)


if __name__ == "__main__":
    unittest.main()

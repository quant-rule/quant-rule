#!/usr/bin/env python3
"""Dry-run tests for the model-cell runner."""

from __future__ import annotations

import csv
import json
import shutil
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

import model_cell

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path(__file__).resolve().parent / "tests" / "fixture_model_shell.py"
EVALUATOR = ROOT / "tools" / "improvement" / "model_prediction.py"
YEARS = (2018, 2019, 2020, 2021, 2022)


def make_cell(root: Path) -> tuple[Path, Path]:
    cell = root / "cells" / "M2.test"
    cell.mkdir(parents=True)
    for instrument in model_cell.INSTRUMENTS:
        (cell / f"spec-{instrument}.json").write_text("{}\n", encoding="utf-8")
        for year in YEARS:
            run = root / "runs" / instrument / str(year) / "run.json"
            run.parent.mkdir(parents=True)
            (run.parent / "data.json").write_text(
                json.dumps({"from": f"{year}-01-01"}), encoding="utf-8"
            )
            run.write_text(json.dumps({"data": "data.json"}), encoding="utf-8")
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
            self.assertTrue((out / "XAU" / "2018" / "model" / "prediction.csv").is_file())
            self.assertFalse((out / "XAU" / "2021").exists())
            merged = json.loads((out / "XAU" / "model" / "merge.json").read_text())
        self.assertEqual(first, second)
        self.assertEqual(document["phase"], "dev")
        self.assertEqual(document["partition"], "model_dev")
        self.assertEqual(document["years"], [2018, 2019, 2020])
        self.assertEqual(document["spec_id"], "M2.test")
        self.assertEqual(len(document["prediction_digest"]), 64)
        self.assertEqual(
            set(document["metrics"]["instruments"]),
            set(model_cell.INSTRUMENTS),
        )
        self.assertEqual(merged["years"], [2018, 2019, 2020])
        self.assertGreaterEqual(document["metrics"]["n"], 200)
        self.assertIn(document["grade"], {"A", "B"})
        self.assertEqual(document["verdict"], "pass")

    def test_state_gate_is_not_passed_to_the_model(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cell, qr = make_cell(root)
            state = cell / "state.json"
            state.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.model-state-gate/v1",
                        "all": [{"column": "level_touches", "min": 1}],
                    }
                ),
                encoding="utf-8",
            )
            out = root / "out"
            code = model_cell.main(
                [
                    "--cell-dir",
                    str(cell),
                    "--phase",
                    "dev",
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
                    "--state-file",
                    str(state),
                ]
            )
            self.assertEqual(code, 0)
            args = json.loads(
                (out / "XAU" / "2018" / "model" / "args.json").read_text()
            )
            self.assertNotIn("--state-file", args)
            self.assertIn("--config", args)
            self.assertIn("--spec", args)

    def test_validation_uses_the_validation_years(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cell, qr = make_cell(root)
            out = root / "out"
            self.assertEqual(run(cell, qr, out, "validation"), 0)
            document = json.loads((out / "model_prediction.json").read_text())
            self.assertTrue((out / "XAU" / "2021" / "model" / "prediction.csv").is_file())
            self.assertFalse((out / "XAU" / "2023").exists())
            with (out / "XAU" / "model" / "prediction.csv").open() as handle:
                rows = list(csv.DictReader(handle))
        self.assertEqual(document["phase"], "validation")
        self.assertEqual(document["partition"], "model_validation")
        self.assertEqual(document["years"], [2021, 2022])
        self.assertGreater(document["metrics"]["n"], 0)
        self.assertEqual(set(document["metrics"]["folds"]), {"2021", "2022"})
        self.assertEqual(
            {
                str(datetime.fromtimestamp(int(row["timestamp"]), timezone.utc).year)
                for row in rows
                if int(row["prediction"]) != 0
            },
            {"2021", "2022"},
        )

    def test_a_duplicate_fold_timestamp_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            for name in ("prediction.csv", "readings.csv"):
                (source / name).write_text(
                    "timeframe_secs,bar,timestamp\n3600,0,1514764800\n",
                    encoding="utf-8",
                )
            with self.assertRaises(SystemExit):
                model_cell.merge_bundles(
                    root / "out", [(2018, source), (2019, source)]
                )


if __name__ == "__main__":
    unittest.main()

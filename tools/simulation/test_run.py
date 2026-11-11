#!/usr/bin/env python3
"""End-to-end test for one simulation run on the fixture bundle."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import run
from render import Image

FIXTURE = Path(__file__).parent / "fixtures" / "mini"


def write_config(path: Path, out: Path) -> None:
    config = {
        "schema": "quantrule.simulation-run/v1",
        "run_id": "SIM-TEST",
        "premise": "Fixture run for the harness test.",
        "bundle": str(FIXTURE),
        "timeframe": "15m",
        "timeframe_secs": 900,
        "from": "2018-01-01",
        "to": "2018-01-05",
        "events": ["level.fib_test"],
        "window_bars": 200,
        "candidate_zone": [60, 100],
        "max_windows": 3,
        "min_windows": 1,
        "seed": 0,
        "overlap_bars": 120,
        "studies": ["ema", "rsi"],
        "oracle": {"entry_offset_max": 0, "horizon_bars": 40},
        "strict_m13": True,
        "theme": "light",
        "title": "Fixture 15m",
        "date": "2026-11-10",
        "check_window": {"from": "2020-01-01", "to": "2022-12-31"},
    }
    path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


@unittest.skipIf(Image is None, "Pillow is not installed")
class RunTest(unittest.TestCase):
    def test_run_writes_the_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "config.json"
            out = root / "run"
            write_config(config, out)
            code = run.main(["run.py", "--run-config", str(config), "--out", str(out)])
            self.assertEqual(code, 0)
            for name in (
                "manifest.json",
                "windows.json",
                "marks-ai.csv",
                "marks-detail.csv",
                "review.md",
                "reproduce.md",
                "decision.md",
                "contact-sheet.png",
                "charts/manifest.json",
                "charts/index.csv",
            ):
                self.assertTrue((out / name).is_file(), name)
            manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["model"], "M1.3")
            self.assertEqual(manifest["run_id"], "SIM-TEST")
            self.assertEqual(manifest["selected_count"], 3)
            self.assertFalse(manifest["underpowered"])
            self.assertEqual(manifest["holdout_access"], "none")
            self.assertTrue(list((out / "charts").glob("*-zoom.png")))
            marks = (out / "marks-ai.csv").read_text(encoding="utf-8")
            self.assertIn("entry", marks)
            self.assertIn("exit", marks)

    def test_run_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "config.json"
            write_config(config, root / "one")
            self.assertEqual(
                run.main(["run.py", "--run-config", str(config), "--out", str(root / "one")]),
                0,
            )
            self.assertEqual(
                run.main(["run.py", "--run-config", str(config), "--out", str(root / "two")]),
                0,
            )
            self.assertEqual(
                (root / "one" / "windows.json").read_bytes(),
                (root / "two" / "windows.json").read_bytes(),
            )
            self.assertEqual(
                (root / "one" / "marks-ai.csv").read_bytes(),
                (root / "two" / "marks-ai.csv").read_bytes(),
            )


if __name__ == "__main__":
    unittest.main()

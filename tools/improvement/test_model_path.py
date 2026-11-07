#!/usr/bin/env python3
"""Tests for the descriptive model path evaluator."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import model_path


def write_bundle(root: Path) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    with (root / "readings.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            ["timeframe_secs", "bar", "timestamp", "open", "high", "low", "close", "volume", "atr"]
        )
        for bar in range(30):
            close = 100.0 + bar * 0.1
            writer.writerow([900, bar, 1_700_000_000 + bar * 900, close, close + 0.5, close - 0.5, close, 1.0, 1.0])
    with (root / "prediction.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["timeframe_secs", "bar", "timestamp", "prediction", "long_votes", "short_votes"])
        writer.writerow([900, 0, 1_700_000_000, 1, 1, 0])
    return root


class PathTests(unittest.TestCase):
    def test_a_reached_target_is_counted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = write_bundle(root / "bundle")
            menu = root / "menu.json"
            menu.write_text(
                json.dumps([{"target_atr": 2.0, "stop_atr": 1.0, "horizon": 25}]),
                encoding="utf-8",
            )
            out = root / "out"
            code = model_path.main(
                [
                    "--bundle",
                    f"XAU={bundle}",
                    "--spec-id",
                    "MR5.test",
                    "--partition",
                    "model_dev",
                    "--menu",
                    str(menu),
                    "--out",
                    str(out),
                ]
            )
            self.assertEqual(code, 0)
            document = json.loads((out / "path.json").read_text())
            summary = document["summaries"][0]["summary"]
            self.assertEqual(summary["n"], 1)
            self.assertEqual(summary["target_count"], 1)
            self.assertEqual(summary["stop_count"], 0)
            self.assertGreater(summary["mean_net_r"], 0.0)
            self.assertGreaterEqual(summary["mean_mfe_atr"], 2.0)

    def test_path_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = write_bundle(root / "bundle")
            menu = root / "menu.json"
            menu.write_text(
                json.dumps([{"target_atr": 2.0, "stop_atr": 1.0, "horizon": 25}]),
                encoding="utf-8",
            )
            first = root / "first"
            second = root / "second"
            for out in (first, second):
                model_path.main(
                    [
                        "--bundle",
                        f"XAU={bundle}",
                        "--spec-id",
                        "MR5.test",
                        "--menu",
                        str(menu),
                        "--out",
                        str(out),
                    ]
                )
            self.assertEqual(
                (first / "path.json").read_bytes(),
                (second / "path.json").read_bytes(),
            )


if __name__ == "__main__":
    unittest.main()

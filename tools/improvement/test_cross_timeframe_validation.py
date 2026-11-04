#!/usr/bin/env python3
"""Unit tests for the cross-timeframe path gate."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import cross_timeframe_validation as gate


def write_readings(path: Path, rows: list[tuple[int, dict]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=[
                "timeframe_secs",
                "bar",
                "timestamp",
                "high",
                "low",
                "close",
                "atr",
                "ema_stack",
                "adx_regime",
            ],
        )
        writer.writeheader()
        for timeframe, row in rows:
            writer.writerow({"timeframe_secs": timeframe, **row})


def write_events(path: Path, events: list[dict]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=[
                "timeframe_secs",
                "event_index",
                "event_type_code",
                "event_subtype_code",
                "source_code",
                "occurred_bar_index",
                "available_bar_index",
                "direction_code",
                "reference_code",
                "primary_value",
                "reference_value",
                "data_completeness",
            ],
        )
        writer.writeheader()
        for index, event in enumerate(events):
            writer.writerow(
                {
                    "timeframe_secs": 900,
                    "event_index": index,
                    "event_type_code": 3,
                    "event_subtype_code": 6,
                    "source_code": 16,
                    "occurred_bar_index": event["bar"],
                    "available_bar_index": event["bar"],
                    "direction_code": event["direction"],
                    "reference_code": "",
                    "primary_value": "",
                    "reference_value": "",
                    "data_completeness": 1.0,
                }
            )


def fixture(root: Path) -> Path:
    run = root / "run"
    run.mkdir()
    rows = []
    for bar in range(60):
        high = 103.0
        low = 98.0
        if bar == 11:
            low = 100.0
        rows.append(
            {
                "bar": bar,
                "timestamp": bar * 900,
                "high": high,
                "low": low,
                "close": 100.0,
                "atr": 1.0,
                "ema_stack": "",
                "adx_regime": "",
            }
        )
    one_hour = [
        {
            "bar": index,
            "timestamp": index * 3600,
            "high": 101.0,
            "low": 99.0,
            "close": 100.0,
            "atr": 1.0,
            "ema_stack": 1.0,
            "adx_regime": 1.0,
        }
        for index in range(3)
    ]
    write_readings(
        run / "readings.csv",
        [(900, row) for row in rows] + [(3600, row) for row in one_hour],
    )
    write_events(
        run / "events.csv",
        [{"bar": 10, "direction": 1}, {"bar": 20, "direction": -1}],
    )
    return run


class AlignmentTests(unittest.TestCase):
    def test_last_closed_one_hour_bar(self) -> None:
        stamps = [0, 3600, 7200]
        self.assertEqual(gate.last_closed_index(stamps, 9000), 1)
        self.assertEqual(gate.last_closed_index(stamps, 7200), 1)
        self.assertEqual(gate.last_closed_index(stamps, 3600), 0)
        self.assertIsNone(gate.last_closed_index(stamps, 0))


class BundleTests(unittest.TestCase):
    def test_the_1h_filter_selects_only_agreeing_events(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            run = fixture(root)
            cell = gate.Cell(
                instrument="TEST",
                fold=2020,
                partition="model_dev",
                config="adx25",
                run=run,
            )
            family = {
                "id": "trend_cont_15m_1h_trend",
                "event_type": 3,
                "subtype": 6,
                "source": 16,
                "context": {"timeframe": "1h", "ema_stack": "agree", "adx_regime": 1},
            }
            bundle = gate.load_bundle(cell, family, gate.Options())
            self.assertIsNotNone(bundle)
            self.assertEqual(len(bundle.prepared), 2)
            self.assertEqual(len(bundle.filtered), 1)
            metrics = gate.evaluate([bundle], family["id"], gate.Options(), 1)
            self.assertEqual(metrics["n"], 1)
            self.assertAlmostEqual(metrics["hit_rate"], 1.0)
            self.assertAlmostEqual(metrics["event_rate"], 0.5)
            self.assertAlmostEqual(metrics["conditional_lift"], 0.5)
            self.assertGreater(metrics["incremental_lift"], 0.0)


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the model-prediction evaluator."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import model_prediction


def write_bundle(root: Path, predictions: list[dict], readings: list[dict]) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    with (root / "prediction.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "timeframe_secs",
                "bar",
                "timestamp",
                "prediction",
                "long_votes",
                "short_votes",
            ]
        )
        for row in predictions:
            writer.writerow(
                [
                    row["timeframe_secs"],
                    row["bar"],
                    row["timestamp"],
                    row["prediction"],
                    row.get("long_votes", ""),
                    row.get("short_votes", ""),
                ]
            )
    with (root / "readings.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "timeframe_secs",
                "bar",
                "timestamp",
                "open",
                "high",
                "low",
                "close",
                "volume",
                "atr",
            ]
        )
        for row in readings:
            writer.writerow(
                [
                    row["timeframe_secs"],
                    row["bar"],
                    row["timestamp"],
                    row["close"],
                    row["high"],
                    row["low"],
                    row["close"],
                    1.0,
                    row.get("atr", 1.0),
                ]
            )
    return root


def known_edge(count: int = 80) -> tuple[list[dict], list[dict]]:
    predictions = {}
    for index in range(0, count, 3):
        predictions[index] = 1
    for index in range(1, count, 3):
        predictions[index] = -1
    close = [100.0]
    for index in range(1, count + 2):
        previous = index - 1
        if previous in predictions:
            close.append(close[-1] + predictions[previous] * 2.0)
        else:
            close.append(close[-1] + (1.0 if previous % 2 == 0 else -1.0))
    readings = [
        {
            "timeframe_secs": 3600,
            "bar": index,
            "timestamp": 1_700_000_000 + index * 3600,
            "close": close[index],
            "high": close[index] + 0.5,
            "low": close[index] - 0.5,
            "atr": 1.0,
        }
        for index in range(count + 1)
    ]
    rows = [
        {
            "timeframe_secs": 3600,
            "bar": index,
            "timestamp": readings[index]["timestamp"],
            "prediction": predictions.get(index, 0),
        }
        for index in range(count)
    ]
    return rows, readings


class OutcomeTests(unittest.TestCase):
    def test_long_target_hits(self) -> None:
        rows = [
            {"close": 100.0, "high": 100.5, "low": 99.5, "atr": 1.0},
            {"close": 101.0, "high": 102.1, "low": 100.5, "atr": 1.0},
        ]
        outcome = model_prediction.barrier_outcome(rows, 0, 1, 1, 2.0, 1.0, 0.0)
        self.assertIsNotNone(outcome)
        self.assertTrue(outcome.hit)
        self.assertEqual(outcome.kind, "target")

    def test_same_bar_stop_wins(self) -> None:
        rows = [
            {"close": 100.0, "high": 100.5, "low": 99.5, "atr": 1.0},
            {"close": 100.0, "high": 102.0, "low": 98.0, "atr": 1.0},
        ]
        outcome = model_prediction.barrier_outcome(rows, 0, 1, 1, 2.0, 1.0, 0.0)
        self.assertIsNotNone(outcome)
        self.assertFalse(outcome.hit)
        self.assertEqual(outcome.kind, "stop")

    def test_cost_can_turn_a_target_into_a_miss(self) -> None:
        rows = [
            {"close": 100.0, "high": 100.5, "low": 99.5, "atr": 1.0},
            {"close": 101.0, "high": 100.6, "low": 100.0, "atr": 1.0},
        ]
        outcome = model_prediction.barrier_outcome(rows, 0, 1, 1, 0.5, 1.0, 100.0)
        self.assertIsNotNone(outcome)
        self.assertFalse(outcome.hit)

    def test_forward_cost_hurdle(self) -> None:
        rows = [
            {"close": 100.0, "high": 100.5, "low": 99.5, "atr": 1.0},
            {"close": 100.5, "high": 101.0, "low": 100.0, "atr": 1.0},
        ]
        gross = model_prediction.forward_outcome(rows, 0, 1, 1, 0.0)
        net = model_prediction.forward_outcome(rows, 0, 1, 1, 100.0)
        self.assertTrue(gross.hit)
        self.assertFalse(net.hit)

    def test_short_history_is_unavailable(self) -> None:
        rows = [{"close": 100.0, "high": 100.5, "low": 99.5, "atr": 1.0}]
        self.assertIsNone(model_prediction.barrier_outcome(rows, 0, 1, 1, 1.0, 1.0, 0.0))
        self.assertIsNone(model_prediction.forward_outcome(rows, 0, 1, 1, 0.0))


class EvaluationTests(unittest.TestCase):
    def run_eval(self, root: Path, name: str, *extra: str) -> dict:
        out = root / "out"
        code = model_prediction.main(
            [
                "--predictions",
                str(root / "prediction.csv"),
                "--readings",
                str(root / "readings.csv"),
                "--instrument",
                name,
                "--partition",
                "model_dev",
                "--outcome",
                "forward",
                "--horizon",
                "1",
                "--cost-bps",
                "0",
                "--random-draws",
                "50",
                "--shuffle-draws",
                "50",
                "--min-events",
                "10",
                "--min-clusters",
                "1",
                "--min-instruments",
                "1",
                "--primary-cells",
                "1",
                "--out",
                str(out),
                *extra,
            ]
        )
        self.assertEqual(code, 0)
        return json.loads((out / "model_prediction.json").read_text(encoding="utf-8"))

    def test_a_known_edge_passes_and_is_deterministic(self) -> None:
        predictions, readings = known_edge()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root, predictions, readings)
            first = self.run_eval(root, "BTC")
            second = self.run_eval(root, "BTC")
        self.assertEqual(first, second)
        metrics = first["metrics"]
        self.assertEqual(metrics["n"], 54)
        self.assertAlmostEqual(metrics["hit_rate"], 1.0)
        self.assertGreater(metrics["lift"], 0.4)
        self.assertIn(first["grade"], {"A", "B"})
        self.assertEqual(first["verdict"], "pass")
        self.assertLessEqual(metrics["shuffle_p"], 0.05)
        self.assertEqual(metrics["instruments"]["BTC"]["n"], 54)
        self.assertIn("2023", metrics["folds"])

    def test_a_lagged_prediction_fails(self) -> None:
        predictions, readings = known_edge()
        lagged = [
            {**row, "prediction": (-row["prediction"] if row["prediction"] else 0)}
            for row in predictions
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root, lagged, readings)
            document = self.run_eval(root, "BTC")
        self.assertEqual(document["grade"], "D")
        self.assertEqual(document["verdict"], "fail")
        self.assertLess(document["metrics"]["lift"], 0.0)

    def test_two_bundles_report_per_instrument_replication(self) -> None:
        primary, primary_readings = known_edge()
        secondary, secondary_readings = known_edge(10)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root / "BTC", primary, primary_readings)
            write_bundle(root / "ETH", secondary, secondary_readings)
            out = root / "out"
            code = model_prediction.main(
                [
                    "--bundle",
                    f"BTC={root / 'BTC'}",
                    "--bundle",
                    f"ETH={root / 'ETH'}",
                    "--partition",
                    "model_dev",
                    "--outcome",
                    "forward",
                    "--horizon",
                    "1",
                    "--cost-bps",
                    "0",
                    "--random-draws",
                    "50",
                    "--shuffle-draws",
                    "50",
                    "--min-events",
                    "10",
                    "--min-clusters",
                    "1",
                    "--min-instruments",
                    "2",
                    "--primary-cells",
                    "1",
                    "--out",
                    str(out),
                ]
            )
            self.assertEqual(code, 0)
            document = json.loads(
                (out / "model_prediction.json").read_text(encoding="utf-8")
            )
        self.assertEqual(document["instrument"], "pooled")
        self.assertEqual(set(document["metrics"]["instruments"]), {"BTC", "ETH"})
        self.assertEqual(document["metrics"]["replication"], 1)
        self.assertEqual(document["grade"], "B")

    def test_a_split_end_purges_crossing_outcomes(self) -> None:
        predictions, readings = known_edge()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root, predictions, readings)
            document = self.run_eval(root, "BTC", "--split-end", "2023-11-15")
        self.assertGreater(document["metrics"]["purged"], 0)
        self.assertLess(document["metrics"]["n"], 54)
        self.assertEqual(document["split_end"], "2023-11-15")

    def test_an_empty_prediction_is_a_fail_not_a_crash(self) -> None:
        predictions = [
            {"timeframe_secs": 3600, "bar": 0, "timestamp": 1, "prediction": 0}
        ]
        readings = [
            {"timeframe_secs": 3600, "bar": 0, "timestamp": 1, "close": 100.0,
             "high": 100.5, "low": 99.5, "atr": 1.0}
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root, predictions, readings)
            document = self.run_eval(root, "BTC")
        self.assertEqual(document["metrics"]["n"], 0)
        self.assertEqual(document["grade"], "D")
        self.assertEqual(document["verdict"], "fail")


if __name__ == "__main__":
    unittest.main()

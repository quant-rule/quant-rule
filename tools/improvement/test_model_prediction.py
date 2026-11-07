#!/usr/bin/env python3
"""Unit tests for the model-prediction evaluator."""

from __future__ import annotations

import csv
import json
import argparse
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


def write_state_bundle(
    root: Path,
    predictions: list[dict],
    readings: list[dict],
) -> Path:
    write_bundle(root, predictions, readings)
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
                "context_code",
                "adx_regime",
                "ema_stack",
                "level_touches",
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
                    row.get("context_code", ""),
                    row.get("adx_regime", ""),
                    row.get("ema_stack", ""),
                    row.get("level_touches", ""),
                ]
            )
    return root


def state_dataset(count: int = 80, state_one_hits: bool = False) -> tuple[list[dict], list[dict]]:
    prediction_by_bar: dict[int, int] = {}
    state_by_bar: dict[int, int] = {}
    for cycle in range(0, count, 8):
        for offset, state, direction in (
            (1, 2, 1),
            (2, 2, -1),
            (5, 1, 1),
            (6, 1, -1),
        ):
            bar = cycle + offset
            if bar < count:
                prediction_by_bar[bar] = direction
                state_by_bar[bar] = state
    close_values: list[float] = []
    close = 100.0
    for bar in range(count + 2):
        close_values.append(close)
        direction = prediction_by_bar.get(bar)
        if direction is None:
            continue
        state = state_by_bar[bar]
        hit = state == 2 or state_one_hits
        if hit:
            close += 2.0 * direction
        else:
            close -= 2.0 * direction
    readings = [
        {
            "timeframe_secs": 3600,
            "bar": bar,
            "timestamp": 1_700_000_000 + bar * 3600,
            "close": close_values[bar],
            "high": close_values[bar] + 0.5,
            "low": close_values[bar] - 0.5,
            "atr": 1.0,
            "context_code": 2 if bar % 8 < 4 else 1,
            "adx_regime": 1 if bar % 8 < 4 else 0,
            "ema_stack": 1 if bar % 8 < 4 else -1,
            "level_touches": 2 if bar % 8 < 4 else 0,
        }
        for bar in range(count + 1)
    ]
    predictions = [
        {
            "timeframe_secs": 3600,
            "bar": bar,
            "timestamp": readings[bar]["timestamp"],
            "prediction": direction,
        }
        for bar, direction in sorted(prediction_by_bar.items())
    ]
    return predictions, readings


def write_state_file(root: Path, column: str = "context_code", values: list[int] | None = None) -> Path:
    path = root / "state.json"
    path.write_text(
        json.dumps(
            {
                "schema": model_prediction.STATE_SCHEMA,
                "all": [{"column": column, "values": values or [2]}],
            }
        ),
        encoding="utf-8",
    )
    return path


def write_bound_gate(
    root: Path,
    column: str = "level_touches",
    *,
    min_value: int | None = None,
    max_value: int | None = None,
) -> Path:
    clause: dict[str, object] = {"column": column}
    if min_value is not None:
        clause["min"] = min_value
    if max_value is not None:
        clause["max"] = max_value
    path = root / "state.json"
    path.write_text(
        json.dumps(
            {
                "schema": model_prediction.STATE_SCHEMA,
                "all": [clause],
            }
        ),
        encoding="utf-8",
    )
    return path


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


class StateGateTests(unittest.TestCase):
    def run_state_eval(
        self,
        root: Path,
        state_file: Path,
        min_instruments: int = 1,
        out_name: str = "out",
    ) -> dict:
        out = root / out_name
        code = model_prediction.main(
            [
                "--predictions",
                str(root / "prediction.csv"),
                "--readings",
                str(root / "readings.csv"),
                "--instrument",
                "BTC",
                "--partition",
                "model_dev",
                "--outcome",
                "barrier",
                "--horizon",
                "1",
                "--target-atr",
                "2.0",
                "--stop-atr",
                "1.0",
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
                str(min_instruments),
                "--min-instrument-events",
                "1",
                "--primary-cells",
                "1",
                "--state-file",
                str(state_file),
                "--out",
                str(out),
            ]
        )
        self.assertEqual(code, 0)
        return json.loads((out / "model_prediction.json").read_text(encoding="utf-8"))

    def test_parser_accepts_two_clauses_and_rejects_unknown_columns(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write_state_file(root, column="context_code", values=[2])
            gate = model_prediction.load_state_gate(path)
            self.assertEqual(len(gate.clauses), 1)
            self.assertTrue(model_prediction.state_matches({"context_code": "2"}, gate))
            self.assertTrue(model_prediction.state_matches({"context_code": "2.0"}, gate))
            self.assertFalse(model_prediction.state_matches({}, gate))
            self.assertFalse(model_prediction.state_matches({"context_code": "1"}, gate))
            bad = root / "bad.json"
            bad.write_text(
                json.dumps(
                    {
                        "schema": model_prediction.STATE_SCHEMA,
                        "all": [{"column": "not_a_state", "values": [2]}],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaises(ValueError):
                model_prediction.load_state_gate(bad)

    def test_parser_accepts_level_touches_bounds(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate = model_prediction.load_state_gate(write_bound_gate(root, min_value=1))
            self.assertFalse(model_prediction.state_matches({"level_touches": "0"}, gate))
            self.assertFalse(model_prediction.state_matches({"level_touches": ""}, gate))
            self.assertFalse(model_prediction.state_matches({}, gate))
            self.assertTrue(model_prediction.state_matches({"level_touches": "1"}, gate))
            self.assertTrue(model_prediction.state_matches({"level_touches": "1.0"}, gate))
            self.assertTrue(model_prediction.state_matches({"level_touches": "2"}, gate))
            gate = model_prediction.load_state_gate(
                write_bound_gate(root, min_value=1, max_value=2)
            )
            self.assertFalse(model_prediction.state_matches({"level_touches": "0"}, gate))
            self.assertTrue(model_prediction.state_matches({"level_touches": "2"}, gate))
            self.assertFalse(model_prediction.state_matches({"level_touches": "3"}, gate))

    def test_parser_rejects_invalid_bounds(self) -> None:
        invalid = (
            {"column": "level_touches", "values": [1], "min": 1},
            {"column": "level_touches"},
            {"column": "level_touches", "min": 1.5},
            {"column": "level_touches", "min": True},
            {"column": "level_touches", "min": 2, "max": 1},
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, clause in enumerate(invalid):
                path = root / f"bad-{index}.json"
                path.write_text(
                    json.dumps(
                        {
                            "schema": model_prediction.STATE_SCHEMA,
                            "all": [clause],
                        }
                    ),
                    encoding="utf-8",
                )
                with self.assertRaises(ValueError):
                    model_prediction.load_state_gate(path)

    def test_level_gate_filters_the_base_and_reports_increment(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_bound_gate(root, min_value=1)
            document = self.run_state_eval(root, state_file)
        metrics = document["metrics"]
        unconditional = document["unconditional_metrics"]
        self.assertEqual(metrics["scope"], "state")
        self.assertEqual(metrics["n"], 20)
        self.assertGreater(metrics["lift"], unconditional["lift"])
        self.assertGreater(document["state_increment"], 0.0)
        self.assertEqual(document["grade"], "A")

    def test_level_gate_can_match_every_bar(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_bound_gate(root, min_value=0)
            document = self.run_state_eval(root, state_file)
        self.assertEqual(
            document["metrics"]["n"], document["unconditional_metrics"]["n"]
        )
        self.assertEqual(document["state_increment"], 0.0)

    def test_level_gate_can_remove_every_bar(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_bound_gate(root, min_value=99)
            document = self.run_state_eval(root, state_file)
        self.assertEqual(document["metrics"]["n"], 0)
        self.assertEqual(document["grade"], "D")

    def test_level_gate_is_deterministic(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_bound_gate(root, min_value=1)
            first = self.run_state_eval(root, state_file, out_name="first")
            second = self.run_state_eval(root, state_file, out_name="second")
        self.assertEqual(first, second)

    def test_state_gate_filters_the_base_and_reports_increment(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_state_file(root)
            document = self.run_state_eval(root, state_file)
        metrics = document["metrics"]
        unconditional = document["unconditional_metrics"]
        self.assertEqual(metrics["scope"], "state")
        self.assertEqual(metrics["n"], 20)
        self.assertEqual(unconditional["n"], 40)
        self.assertGreater(metrics["lift"], unconditional["lift"])
        self.assertGreater(document["state_increment"], 0.0)
        self.assertEqual(document["grade"], "A")
        self.assertEqual(document["verdict"], "pass")
        self.assertTrue(document["criteria_status"]["state_increment"])

    def test_state_gated_grade_b_does_not_promote(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_state_file(root)
            document = self.run_state_eval(root, state_file, min_instruments=2)
        self.assertEqual(document["grade"], "B")
        self.assertEqual(document["verdict"], "fail")
        self.assertFalse(document["criteria_status"]["replication"])

    def test_state_gate_is_deterministic(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_state_bundle(root, predictions, readings)
            state_file = write_state_file(root)
            first = self.run_state_eval(root, state_file, out_name="first")
            second = self.run_state_eval(root, state_file, out_name="second")
        self.assertEqual(first, second)

    def test_missing_state_column_is_an_error(self) -> None:
        predictions, readings = state_dataset()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root, predictions, readings)
            state_file = write_state_file(root)
            code = model_prediction.main(
                [
                    "--predictions",
                    str(root / "prediction.csv"),
                    "--readings",
                    str(root / "readings.csv"),
                    "--instrument",
                    "BTC",
                    "--partition",
                    "model_dev",
                    "--outcome",
                    "barrier",
                    "--horizon",
                    "1",
                    "--state-file",
                    str(state_file),
                    "--out",
                    str(root / "out"),
                ]
            )
        self.assertEqual(code, 1)

    def test_state_increment_is_a_required_gate(self) -> None:
        metrics = {
            "n": 300,
            "clusters": 30,
            "lift": 0.1,
            "lift_lcb": 0.01,
            "random_p": 0.001,
            "shuffle_p": 0.001,
            "back_lag_lift": 0.0,
            "replication": 2,
        }
        args = argparse.Namespace(
            min_events=200,
            min_clusters=20,
            min_lift=0.02,
            min_instruments=2,
            min_state_increment=0.01,
        )
        letter, passed, criteria = model_prediction.grade(
            metrics, args, 0.01, state_increment=0.0, require_replication=True
        )
        self.assertEqual(letter, "C")
        self.assertFalse(passed)
        self.assertFalse(criteria["state_increment"])
        letter, passed, criteria = model_prediction.grade(
            metrics, args, 0.01, state_increment=0.02, require_replication=True
        )
        self.assertEqual(letter, "A")
        self.assertTrue(passed)
        self.assertTrue(criteria["state_increment"])


if __name__ == "__main__":
    unittest.main()

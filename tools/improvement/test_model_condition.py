#!/usr/bin/env python3
"""Tests for the model condition preparer."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import model_condition

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "docs/registry/indicators.json"


def write_readings(root: Path, rows: list[dict[str, object]]) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    path = root / "readings.csv"
    with path.open("w", newline="", encoding="utf-8") as handle:
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
                "level_touches",
                "level_age",
                "vwap",
                "poc",
                "vah",
                "val",
                "last_pivot_high",
                "last_pivot_low",
            ]
        )
        for row in rows:
            writer.writerow(
                [
                    900,
                    row["bar"],
                    1_700_000_000 + int(row["bar"]) * 900,
                    row["close"],
                    row["close"] + 0.5,
                    row["close"] - 0.5,
                    row["close"],
                    1.0,
                    1.0,
                    row.get("level_touches", 0),
                    row.get("level_age", 99),
                    row.get("vwap", ""),
                    row.get("poc", ""),
                    row.get("vah", ""),
                    row.get("val", ""),
                    row.get("last_pivot_high", ""),
                    row.get("last_pivot_low", ""),
                ]
            )
    with (root / "prediction.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["timeframe_secs", "bar", "timestamp", "prediction", "long_votes", "short_votes"])
        for row in rows:
            writer.writerow([900, row["bar"], 1_700_000_000 + int(row["bar"]) * 900, 0, 0, 0])
    return path


def write_events(root: Path, rows: list[dict[str, object]]) -> Path:
    path = root / "events.csv"
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "timeframe_secs",
                "event_index",
                "event_code",
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
            ]
        )
        for index, row in enumerate(rows):
            writer.writerow(
                [
                    900,
                    index,
                    row["code"],
                    1,
                    1,
                    1,
                    row["bar"],
                    row["bar"],
                    row["direction"],
                    row.get("reference_code", ""),
                    "",
                    row.get("reference_value", ""),
                    1,
                ]
            )
    return path


def write_condition(root: Path, document: dict) -> Path:
    path = root / "condition.json"
    path.write_text(json.dumps(document), encoding="utf-8")
    return path


def base_readings() -> list[dict[str, object]]:
    return [
        {
            "bar": bar,
            "close": 100.0 + bar * 0.1,
            "level_touches": 1 if bar % 3 == 0 else 0,
            "level_age": bar % 4,
            "vwap": 100.0,
            "poc": 100.1,
            "vah": 105.0,
            "val": 95.0,
        }
        for bar in range(12)
    ]


class ConditionTests(unittest.TestCase):
    def test_sequence_failed_breakout_sets_the_bar(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            readings = base_readings()
            readings[4]["close"] = 100.5
            write_readings(root, readings)
            write_events(
                root,
                [
                    {"code": 3, "bar": 2, "direction": 1, "reference_code": "1001", "reference_value": 101.0},
                    {"code": 2, "bar": 4, "direction": -1, "reference_code": "1001", "reference_value": 101.0},
                ],
            )
            condition = write_condition(
                root,
                {
                    "schema": model_condition.SCHEMA,
                    "kind": "sequence",
                    "first_event": "level.break",
                    "second_event": "level.cross",
                    "within_bars": 3,
                    "same_level": True,
                    "direction": "opposite",
                    "close_side": "original",
                },
            )
            out = root / "out"
            code = model_condition.main(
                [
                    "--bundle",
                    str(root),
                    "--condition",
                    str(condition),
                    "--registry",
                    str(REGISTRY),
                    "--out",
                    str(out),
                ]
            )
            self.assertEqual(code, 0)
            document = json.loads((out / "condition.json").read_text())
            self.assertEqual(document["kind"], "sequence")
            self.assertEqual(document["condition_bars"], 1)
            header, rows = model_condition.read_csv(out / "readings.csv")
            index = header.index("condition")
            by_bar = {int(row[1]): int(row[index]) for row in rows}
            self.assertEqual(by_bar[4], 1)
            self.assertEqual(sum(by_bar.values()), 1)
            gate = json.loads((out / "state.json").read_text())
            self.assertEqual(gate["all"][0]["column"], "condition")

    def test_confluence_counts_independent_references(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_readings(root, base_readings())
            condition = write_condition(
                root,
                {
                    "schema": model_condition.SCHEMA,
                    "kind": "confluence",
                    "references": ["vwap", "poc"],
                    "min_count": 2,
                    "within_atr": 2.0,
                },
            )
            out = root / "out"
            code = model_condition.main(
                ["--bundle", str(root), "--condition", str(condition), "--out", str(out)]
            )
            self.assertEqual(code, 0)
            document = json.loads((out / "condition.json").read_text())
            self.assertGreater(document["condition_bars"], 0)
            self.assertEqual(document["condition_bars"], 12)

    def test_column_freshness(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_readings(root, base_readings())
            condition = write_condition(
                root,
                {
                    "schema": model_condition.SCHEMA,
                    "kind": "column",
                    "column": "level_touches",
                    "values": [1],
                    "level_age_max": 1,
                },
            )
            out = root / "out"
            code = model_condition.main(
                ["--bundle", str(root), "--condition", str(condition), "--out", str(out)]
            )
            self.assertEqual(code, 0)
            header, rows = model_condition.read_csv(out / "readings.csv")
            index = header.index("condition")
            for row in rows:
                bar = int(row[1])
                touches = int(row[header.index("level_touches")])
                age = int(row[header.index("level_age")])
                expected = 1 if touches == 1 and age <= 1 else 0
                self.assertEqual(int(row[index]), expected)

    def test_a_later_bar_does_not_change_a_past_match(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            readings = base_readings()
            readings[4]["close"] = 100.5
            write_readings(root, readings)
            write_events(
                root,
                [
                    {"code": 3, "bar": 2, "direction": 1, "reference_code": "1001", "reference_value": 101.0},
                    {"code": 2, "bar": 4, "direction": -1, "reference_code": "1001", "reference_value": 101.0},
                ],
            )
            condition = write_condition(
                root,
                {
                    "schema": model_condition.SCHEMA,
                    "kind": "sequence",
                    "first_event": "level.break",
                    "second_event": "level.cross",
                    "within_bars": 3,
                    "same_level": True,
                    "direction": "opposite",
                    "close_side": "original",
                },
            )
            first = root / "first"
            second = root / "second"
            model_condition.main(
                ["--bundle", str(root), "--condition", str(condition), "--registry", str(REGISTRY), "--out", str(first)]
            )
            changed = base_readings()
            changed[4]["close"] = 100.5
            changed[9]["close"] = 999.0
            write_readings(root, changed)
            model_condition.main(
                ["--bundle", str(root), "--condition", str(condition), "--registry", str(REGISTRY), "--out", str(second)]
            )
            first_rows = model_condition.read_csv(first / "readings.csv")[1]
            second_rows = model_condition.read_csv(second / "readings.csv")[1]
            first_index = model_condition.read_csv(first / "readings.csv")[0].index("condition")
            second_index = model_condition.read_csv(second / "readings.csv")[0].index("condition")
            first_by_bar = {int(row[1]): int(row[first_index]) for row in first_rows}
            second_by_bar = {int(row[1]): int(row[second_index]) for row in second_rows}
            self.assertEqual(first_by_bar[4], second_by_bar[4])

    def test_condition_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_readings(root, base_readings())
            condition = write_condition(
                root,
                {
                    "schema": model_condition.SCHEMA,
                    "kind": "confluence",
                    "references": ["vwap", "poc"],
                    "min_count": 2,
                    "within_atr": 0.5,
                },
            )
            first = root / "first"
            second = root / "second"
            model_condition.main(["--bundle", str(root), "--condition", str(condition), "--out", str(first)])
            model_condition.main(["--bundle", str(root), "--condition", str(condition), "--out", str(second)])
            self.assertEqual(
                (first / "readings.csv").read_bytes(),
                (second / "readings.csv").read_bytes(),
            )
            self.assertEqual(
                (first / "condition.json").read_bytes(),
                (second / "condition.json").read_bytes(),
            )


if __name__ == "__main__":
    unittest.main()

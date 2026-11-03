#!/usr/bin/env python3
"""Unit tests for the ESANDP500 pivot-level structure pilot."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import structure_pilot

HEADER = [
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
]


def event(
    bar: int,
    reference: int,
    direction: int = 1,
    subtype: str = "1",
    source: str = "7",
    event_index: int = 0,
) -> dict:
    return {
        "bar": bar,
        "subtype": subtype,
        "source": source,
        "reference": reference,
        "direction": direction,
        "event_index": event_index,
    }


def write_events(path: Path, rows: list[dict]) -> None:
    with path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(HEADER)
        for index, row in enumerate(rows):
            writer.writerow(
                [
                    3600,
                    row.get("event_index", index),
                    row.get("type", 1),
                    row.get("subtype", "1"),
                    row.get("source", "7"),
                    row.get("bar", 0),
                    row.get("bar", 0),
                    row.get("direction", 1),
                    row.get("reference", 1001),
                    "",
                    "",
                    row.get("completeness", 1),
                ]
            )


def cell(n: int, lift: float, lift_lcb: float) -> dict:
    return {"n": n, "lift": lift, "lift_lcb": lift_lcb}


def partition(value: dict) -> dict:
    return {"cells": {"test": value}}


class EventFilterTests(unittest.TestCase):
    def test_only_confirmed_pivot_level_events_are_kept(self) -> None:
        rows = [
            {"subtype": "1", "source": "7", "reference": 1001, "direction": 1},
            {"subtype": "2", "source": "8", "reference": 1002, "direction": -1},
            {"subtype": "1", "source": "1", "reference": 1, "direction": 1},
            {"subtype": "1", "source": "7", "reference": 1000, "direction": 1},
            {"subtype": "1", "source": "7", "reference": 1001, "direction": 0},
            {"subtype": "1", "source": "7", "reference": 1001, "direction": 1, "completeness": 0.5},
            {"subtype": "1", "source": "7", "reference": 1001, "direction": 1, "type": 2},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.csv"
            write_events(path, rows)
            events = structure_pilot.read_events(path)
        self.assertEqual(len(events), 2)
        self.assertEqual({row["subtype"] for row in events}, {"1", "2"})

    def test_a_fresh_test_starts_after_a_gap_for_its_reference(self) -> None:
        events = [
            event(10, 1001),
            event(11, 1001),
            event(12, 1001),
            event(15, 1001),
            event(11, 2001),
        ]
        fresh, prepared, conflicts = structure_pilot.prepare_subtype(events, "1")
        self.assertEqual([row["bar"] for row in fresh], [10, 11, 15])
        self.assertEqual([row["bar"] for row in prepared], [10, 11, 15])
        self.assertEqual(conflicts, 0)

    def test_conflicting_directions_on_one_bar_are_dropped(self) -> None:
        events = [
            event(10, 1001, direction=1, event_index=0),
            event(10, 2001, direction=-1, event_index=1),
            event(11, 1001, direction=1, event_index=2),
        ]
        fresh, prepared, conflicts = structure_pilot.prepare_subtype(events, "1")
        self.assertEqual([row["bar"] for row in fresh], [10, 10])
        self.assertEqual(prepared, [])
        self.assertEqual(conflicts, 1)

    def test_one_vote_per_bar_keeps_the_first_event(self) -> None:
        events = [
            event(10, 1001, event_index=5),
            event(10, 2001, event_index=2),
        ]
        _, prepared, _ = structure_pilot.prepare_subtype(events, "1")
        self.assertEqual(len(prepared), 1)
        self.assertEqual(prepared[0]["reference"], 2001)


class TargetTests(unittest.TestCase):
    def test_forward_uses_the_next_bar_only_when_it_is_one_hour_later(self) -> None:
        rows = [
            {"bar": 0, "timestamp": 0, "close": 100.0},
            {"bar": 1, "timestamp": 3600, "close": 101.0},
            {"bar": 2, "timestamp": 7200, "close": 100.0},
        ]
        self.assertAlmostEqual(structure_pilot.forward(rows, 0), 0.01)
        self.assertAlmostEqual(structure_pilot.forward(rows, 1), -1.0 / 101.0)
        self.assertIsNone(structure_pilot.forward(rows, 2))

    def test_a_gap_is_not_a_one_hour_return(self) -> None:
        rows = [
            {"bar": 0, "timestamp": 0, "close": 100.0},
            {"bar": 1, "timestamp": 7200, "close": 101.0},
        ]
        self.assertIsNone(structure_pilot.forward(rows, 0))
        self.assertIsNone(structure_pilot.forward(rows, 1))

    def test_base_rate_uses_valid_hourly_transitions(self) -> None:
        rows = [
            {"bar": 0, "timestamp": 0, "close": 100.0},
            {"bar": 1, "timestamp": 3600, "close": 101.0},
            {"bar": 2, "timestamp": 7200, "close": 100.0},
            {"bar": 3, "timestamp": 10800, "close": 100.0},
        ]
        stats = structure_pilot.base_stats(rows)
        self.assertEqual(stats["n"], 3)
        self.assertAlmostEqual(stats["up"], 1.0 / 3.0)
        self.assertAlmostEqual(stats["down"], 1.0 / 3.0)

    def test_base_rate_skips_a_gap(self) -> None:
        rows = [
            {"bar": 0, "timestamp": 0, "close": 100.0},
            {"bar": 1, "timestamp": 7200, "close": 101.0},
            {"bar": 2, "timestamp": 10800, "close": 100.0},
        ]
        stats = structure_pilot.base_stats(rows)
        self.assertEqual(stats["n"], 1)
        self.assertAlmostEqual(stats["up"], 0.0)
        self.assertAlmostEqual(stats["down"], 1.0)


class DecisionTests(unittest.TestCase):
    def test_failed_development_stops(self) -> None:
        dev = partition(cell(200, -0.01, -0.02))
        validation = partition(cell(200, 0.05, 0.01))
        self.assertEqual(structure_pilot.decision(dev, validation), "stop")

    def test_too_few_development_events_is_inconclusive(self) -> None:
        dev = partition(cell(50, 0.05, 0.01))
        validation = partition(cell(200, 0.05, 0.01))
        self.assertEqual(structure_pilot.decision(dev, validation), "inconclusive")

    def test_both_partitions_clear_proceed(self) -> None:
        dev = partition(cell(200, 0.03, 0.005))
        validation = partition(cell(200, 0.02, 0.004))
        self.assertEqual(structure_pilot.decision(dev, validation), "proceed_to_progression")

    def test_validation_failure_is_development_only(self) -> None:
        dev = partition(cell(200, 0.03, 0.005))
        validation = partition(cell(200, 0.01, -0.001))
        self.assertEqual(structure_pilot.decision(dev, validation), "development_only")

    def test_too_few_validation_events_is_not_a_pass(self) -> None:
        dev = partition(cell(200, 0.03, 0.005))
        validation = partition(cell(50, 0.03, 0.005))
        self.assertEqual(structure_pilot.decision(dev, validation), "validation_inconclusive")


if __name__ == "__main__":
    unittest.main()

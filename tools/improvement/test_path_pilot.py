#!/usr/bin/env python3
"""Unit tests for the COPPERCMD path prediction pilot."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import path_pilot

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


def candle(bar: int, close: float, high: float, low: float, atr: float | None = 1.0) -> dict:
    return {
        "bar": bar,
        "timestamp": bar * 3600,
        "high": high,
        "low": low,
        "close": close,
        "atr": atr,
    }


def flat_path(length: int, atr: float = 1.0) -> list[dict]:
    rows = [candle(0, 100.0, 100.5, 99.5, atr)]
    rows.extend(candle(index, 100.0, 100.5, 99.5, atr) for index in range(1, length))
    return rows


def cell(n: int, clusters: int, lift: float, lift_lcb: float) -> dict:
    return {"n": n, "clusters": clusters, "lift": lift, "lift_lcb": lift_lcb}


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
            events = path_pilot.read_events(path)
        self.assertEqual(len(events), 2)
        self.assertEqual({row["subtype"] for row in events}, {"1", "2"})

    def test_fresh_test_and_one_vote_per_bar(self) -> None:
        events = [
            event(10, 1001),
            event(11, 1001),
            event(12, 1001),
            event(15, 1001),
            event(11, 2001),
        ]
        fresh, prepared, conflicts = path_pilot.prepare_subtype(events, "1")
        self.assertEqual([row["bar"] for row in fresh], [10, 11, 15])
        self.assertEqual([row["bar"] for row in prepared], [10, 11, 15])
        self.assertEqual(conflicts, 0)

    def test_conflicting_directions_on_one_bar_are_dropped(self) -> None:
        events = [
            event(10, 1001, direction=1, event_index=0),
            event(10, 2001, direction=-1, event_index=1),
            event(11, 1001, direction=1, event_index=2),
        ]
        fresh, prepared, conflicts = path_pilot.prepare_subtype(events, "1")
        self.assertEqual([row["bar"] for row in fresh], [10, 10])
        self.assertEqual(prepared, [])
        self.assertEqual(conflicts, 1)

    def test_one_vote_per_bar_keeps_the_first_event(self) -> None:
        events = [
            event(10, 1001, event_index=5),
            event(10, 2001, event_index=2),
        ]
        _, prepared, _ = path_pilot.prepare_subtype(events, "1")
        self.assertEqual(len(prepared), 1)
        self.assertEqual(prepared[0]["reference"], 2001)


class PathTests(unittest.TestCase):
    def test_long_target(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1] = candle(1, 101.0, 101.5, 99.5)
        outcome = path_pilot.path_outcome(rows, 0, 1)
        self.assertEqual(outcome["status"], "target")
        self.assertEqual(outcome["bars"], 1)

    def test_long_stop(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1] = candle(1, 99.0, 100.5, 98.5)
        outcome = path_pilot.path_outcome(rows, 0, 1)
        self.assertEqual(outcome["status"], "stop")
        self.assertEqual(outcome["bars"], 1)

    def test_short_target(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1] = candle(1, 99.0, 100.5, 98.5)
        outcome = path_pilot.path_outcome(rows, 0, -1)
        self.assertEqual(outcome["status"], "target")
        self.assertEqual(outcome["bars"], 1)

    def test_short_stop(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1] = candle(1, 101.0, 101.5, 99.5)
        outcome = path_pilot.path_outcome(rows, 0, -1)
        self.assertEqual(outcome["status"], "stop")

    def test_same_bar_target_and_stop_is_a_stop(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1] = candle(1, 100.0, 101.5, 98.5)
        outcome = path_pilot.path_outcome(rows, 0, 1)
        self.assertEqual(outcome["status"], "stop")

    def test_timeout(self) -> None:
        rows = flat_path(21)
        outcome = path_pilot.path_outcome(rows, 0, 1)
        self.assertEqual(outcome["status"], "timeout")
        self.assertEqual(outcome["bars"], path_pilot.HORIZON)

    def test_null_and_zero_atr_are_invalid(self) -> None:
        rows = flat_path(21)
        rows[0]["atr"] = None
        self.assertEqual(path_pilot.path_outcome(rows, 0, 1)["status"], "invalid_atr")
        rows[0]["atr"] = 0.0
        self.assertEqual(path_pilot.path_outcome(rows, 0, 1)["status"], "invalid_atr")

    def test_short_history_is_invalid(self) -> None:
        rows = flat_path(10)
        self.assertEqual(path_pilot.path_outcome(rows, 0, 1)["status"], "short_history")

    def test_a_gap_is_just_another_bar(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(flat_path(21)[1:])
        rows[1]["timestamp"] = 10 * 3600
        rows[1] = candle(1, 101.0, 101.5, 99.5)
        rows[1]["timestamp"] = 10 * 3600
        self.assertEqual(path_pilot.path_outcome(rows, 0, 1)["status"], "target")


class ClusterTests(unittest.TestCase):
    def test_paths_that_can_overlap_share_a_cluster(self) -> None:
        clusters = path_pilot.clusters_from_events([(0, 1), (19, 0)])
        self.assertEqual(clusters, [(2, 1)])

    def test_a_path_starting_at_the_horizon_is_a_new_cluster(self) -> None:
        clusters = path_pilot.clusters_from_events([(0, 1), (20, 1), (40, 1)])
        self.assertEqual(clusters, [(1, 1), (1, 1), (1, 1)])

    def test_cluster_statistics(self) -> None:
        stats = path_pilot.cluster_statistics([(2, 1), (2, 2)])
        self.assertEqual(stats["n"], 4)
        self.assertEqual(stats["clusters"], 2)
        self.assertAlmostEqual(stats["hit_rate"], 0.75)
        self.assertAlmostEqual(stats["naive_se"], (0.75 * 0.25 / 4) ** 0.5)
        self.assertAlmostEqual(stats["cluster_se"], 0.25)

    def test_one_cluster_has_no_cluster_bound(self) -> None:
        stats = path_pilot.cluster_statistics([(2, 1)])
        self.assertIsNone(stats["cluster_se"])


class BaseRateTests(unittest.TestCase):
    def test_long_and_short_base_rates_are_counted_separately(self) -> None:
        rows = [candle(0, 100.0, 100.0, 100.0)]
        rows.extend(candle(index, 100.0, 100.5, 99.5) for index in range(1, 21))
        rows[1] = candle(1, 101.0, 101.5, 99.5)
        stats = path_pilot.base_rates(rows)
        self.assertEqual(stats["n"], 1)
        self.assertAlmostEqual(stats["long"], 1.0)
        self.assertAlmostEqual(stats["short"], 0.0)


class BundleAggregationTests(unittest.TestCase):
    def test_clusters_do_not_span_bundles(self) -> None:
        readings_header = [
            "timeframe_secs",
            "bar",
            "timestamp",
            "high",
            "low",
            "close",
            "atr",
        ]

        def write_bundle(directory: Path) -> None:
            directory.mkdir(parents=True)
            with (directory / "readings.csv").open("w", newline="") as handle:
                writer = csv.writer(handle)
                writer.writerow(readings_header)
                for bar in range(21):
                    high = 101.5 if bar == 1 else 100.5
                    low = 99.5 if bar == 1 else 99.5
                    close = 101.0 if bar == 1 else 100.0
                    writer.writerow([3600, bar, bar * 3600, high, low, close, 1.0])
            write_events(
                directory / "events.csv",
                [{"bar": 0, "reference": 1001, "direction": 1}],
            )

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_bundle(root / "a" / "model")
            write_bundle(root / "b" / "model")
            result = path_pilot.summarize_bundles(
                [str(root / "a" / "model"), str(root / "b" / "model")]
            )
        cell = result["cells"]["test"]
        self.assertEqual(cell["n"], 2)
        self.assertEqual(cell["clusters"], 2)


class DecisionTests(unittest.TestCase):
    def test_failed_development_stops(self) -> None:
        dev = partition(cell(200, 20, -0.01, -0.02))
        validation = partition(cell(200, 20, 0.05, 0.01))
        self.assertEqual(path_pilot.decision(dev, validation), "stop")

    def test_too_few_development_events_is_inconclusive(self) -> None:
        dev = partition(cell(100, 20, 0.05, 0.01))
        validation = partition(cell(200, 20, 0.05, 0.01))
        self.assertEqual(path_pilot.decision(dev, validation), "inconclusive")

    def test_too_few_development_clusters_is_inconclusive(self) -> None:
        dev = partition(cell(200, 10, 0.05, 0.01))
        validation = partition(cell(200, 20, 0.05, 0.01))
        self.assertEqual(path_pilot.decision(dev, validation), "inconclusive")

    def test_both_partitions_clear_proceed_to_replication(self) -> None:
        dev = partition(cell(200, 20, 0.03, 0.005))
        validation = partition(cell(200, 20, 0.03, 0.005))
        self.assertEqual(path_pilot.decision(dev, validation), "proceed_to_replication")

    def test_validation_failure_is_development_only(self) -> None:
        dev = partition(cell(200, 20, 0.03, 0.005))
        validation = partition(cell(200, 20, 0.01, -0.001))
        self.assertEqual(path_pilot.decision(dev, validation), "development_only")

    def test_too_few_validation_clusters_is_not_a_pass(self) -> None:
        dev = partition(cell(200, 20, 0.03, 0.005))
        validation = partition(cell(200, 10, 0.03, 0.005))
        self.assertEqual(path_pilot.decision(dev, validation), "validation_inconclusive")


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the single-timeframe prediction pilot."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import prediction_pilot


def document(hit: float, lcb: float, lift: float, n: int = 1000) -> dict:
    cell = {
        "n": n,
        "events": n,
        "coverage": 1.0,
        "hits": int(hit * n),
        "hit_rate": hit,
        "base_rate": hit - lift,
        "lift": lift,
        "lcb": lcb,
    }
    return {"rsi_reversion": {horizon: cell for horizon in prediction_pilot.HORIZONS}}


class EventFilterTests(unittest.TestCase):
    def test_only_reversion_crossings_are_kept(self) -> None:
        rows = [
            ("3", "1", "4", "30", "1", "10"),
            ("3", "1", "4", "30", "-1", "11"),
            ("3", "1", "4", "70", "-1", "12"),
            ("3", "1", "4", "70", "1", "13"),
            ("3", "1", "4", "50", "1", "14"),
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "events.csv"
            with path.open("w", newline="") as handle:
                writer = csv.writer(handle)
                writer.writerow(
                    [
                        "timeframe_secs", "event_index", "event_type_code",
                        "event_subtype_code", "source_code", "occurred_bar_index",
                        "available_bar_index", "direction_code", "reference_code",
                        "primary_value", "reference_value", "data_completeness",
                    ]
                )
                for index, (etype, subtype, source, reference, direction, bar) in enumerate(rows):
                    writer.writerow(
                        [3600, index, etype, subtype, source, bar, bar, direction, "", "", reference, 1]
                    )
            events = prediction_pilot.read_events(path)
        self.assertEqual(len(events), 2)
        self.assertEqual({(event["threshold"], event["direction"]) for event in events}, {(30.0, 1), (70.0, -1)})

    def test_event_hit_uses_the_event_direction(self) -> None:
        self.assertTrue(prediction_pilot.event_hit({"direction": 1}, 0.01))
        self.assertFalse(prediction_pilot.event_hit({"direction": 1}, -0.01))
        self.assertTrue(prediction_pilot.event_hit({"direction": -1}, -0.01))


class DecisionTests(unittest.TestCase):
    def test_both_partitions_clear_proceed(self) -> None:
        dev = document(0.55, 0.52, 0.03)
        val = document(0.54, 0.51, 0.02)
        self.assertEqual(prediction_pilot.decision(dev, val), "proceed_to_macd")

    def test_development_only_is_not_promoted(self) -> None:
        dev = document(0.55, 0.52, 0.03)
        val = document(0.51, 0.49, 0.01)
        self.assertEqual(prediction_pilot.decision(dev, val), "development_only")

    def test_failed_development_stops(self) -> None:
        dev = document(0.48, 0.45, -0.02)
        val = document(0.55, 0.52, 0.03)
        self.assertEqual(prediction_pilot.decision(dev, val), "stop")


if __name__ == "__main__":
    unittest.main()

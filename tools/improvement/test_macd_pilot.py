#!/usr/bin/env python3
"""Unit tests for the XAGUSD MACD prediction pilot."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import macd_pilot


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
    return {"macd_cross": cell}


class EventFilterTests(unittest.TestCase):
    def test_macd_events_are_split_by_subtype_and_source(self) -> None:
        rows = [
            ("2", "13", "1", "10"),
            ("2", "14", "1", "11"),
            ("5", "13", "-1", "12"),
            ("4", "15", "1", "13"),
            ("1", "4", "1", "14"),
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
                for index, (subtype, source, direction, bar) in enumerate(rows):
                    writer.writerow(
                        [3600, index, 3, subtype, source, bar, bar, direction, "", "", "", 1]
                    )
            events = macd_pilot.read_events(path)
        self.assertEqual(len(events["macd_cross"]), 1)
        self.assertEqual(len(events["macd_zero"]), 1)
        self.assertEqual(len(events["macd_histogram"]), 1)


class DecisionTests(unittest.TestCase):
    def test_both_partitions_clear_proceed(self) -> None:
        self.assertEqual(
            macd_pilot.decision(
                document(0.55, 0.52, 0.03),
                document(0.54, 0.51, 0.02),
            ),
            "proceed_to_conjunction",
        )

    def test_development_only_is_not_promoted(self) -> None:
        self.assertEqual(
            macd_pilot.decision(
                document(0.55, 0.52, 0.03),
                document(0.51, 0.49, 0.01),
            ),
            "development_only",
        )

    def test_failed_development_stops(self) -> None:
        self.assertEqual(
            macd_pilot.decision(
                document(0.46, 0.43, -0.04),
                document(0.55, 0.52, 0.03),
            ),
            "stop",
        )


if __name__ == "__main__":
    unittest.main()

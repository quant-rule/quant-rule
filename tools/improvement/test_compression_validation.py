#!/usr/bin/env python3
"""Unit tests for the compression-breakout range harness."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import compression_validation


def row(index: int, high: float, low: float, close: float, atr: float = 1.0, context: float = 2.0) -> dict:
    return {
        "bar": index,
        "high": high,
        "low": low,
        "close": close,
        "atr": atr,
        "rvol": 1.5,
        "context_code": context,
    }


class ForwardHitTests(unittest.TestCase):
    def test_a_wide_forward_window_is_a_hit(self) -> None:
        rows = [row(0, 100.5, 99.5, 100.0)]
        rows += [row(index, 102.0, 98.0, 100.0) for index in range(1, 5)]
        hits = compression_validation.forward_hits(rows, 4, 2.0)
        self.assertTrue(hits[0])

    def test_a_narrow_forward_window_is_a_miss(self) -> None:
        rows = [row(0, 100.5, 99.5, 100.0)]
        rows += [row(index, 100.4, 99.6, 100.0) for index in range(1, 5)]
        hits = compression_validation.forward_hits(rows, 4, 2.0)
        self.assertFalse(hits[0])

    def test_a_missing_atr_is_unavailable(self) -> None:
        rows = [row(0, 100.5, 99.5, 100.0, atr=None)]
        rows += [row(index, 102.0, 98.0, 100.0) for index in range(1, 5)]
        hits = compression_validation.forward_hits(rows, 4, 2.0)
        self.assertIsNone(hits[0])

    def test_a_short_window_is_unavailable(self) -> None:
        rows = [row(0, 100.5, 99.5, 100.0)]
        hits = compression_validation.forward_hits(rows, 4, 2.0)
        self.assertIsNone(hits[0])


class SelectionTests(unittest.TestCase):
    def test_the_event_filter_and_one_vote_per_bar(self) -> None:
        events = [
            {"type": 3, "subtype": 7, "source": 5, "direction": 1, "available": 5},
            {"type": 3, "subtype": 7, "source": 5, "direction": 1, "available": 5},
            {"type": 3, "subtype": 6, "source": 16, "direction": 1, "available": 6},
            {"type": 1, "subtype": 3, "source": 1, "direction": 1, "available": 7},
        ]
        hits = [True] * 10
        selected, raw = compression_validation.select_events(events, hits)
        self.assertEqual(raw, 2)
        self.assertEqual(len(selected), 1)
        self.assertEqual(selected[0]["available"], 5)

    def test_conflicting_directions_are_dropped(self) -> None:
        events = [
            {"type": 3, "subtype": 7, "source": 5, "direction": 1, "available": 5},
            {"type": 3, "subtype": 7, "source": 5, "direction": -1, "available": 5},
        ]
        hits = [True] * 10
        selected, _ = compression_validation.select_events(events, hits)
        self.assertEqual(selected, [])

    def test_an_event_without_a_full_window_is_dropped(self) -> None:
        events = [
            {"type": 3, "subtype": 7, "source": 5, "direction": 1, "available": 9},
        ]
        hits = [True] * 10
        hits[9] = None
        selected, _ = compression_validation.select_events(events, hits)
        self.assertEqual(selected, [])


class EndToEndTests(unittest.TestCase):
    def write_bundle(self, directory: Path) -> None:
        with (directory / "readings.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=[
                    "timeframe_secs",
                    "bar",
                    "high",
                    "low",
                    "close",
                    "atr",
                    "rvol",
                    "context_code",
                ],
            )
            writer.writeheader()
            for index in range(30):
                context = 2.0 if index < 25 else 3.0
                if index == 25:
                    high, low, close = 101.2, 100.8, 101.0
                elif index > 25:
                    high, low, close = 102.5, 98.5, 100.0
                else:
                    high, low, close = 100.5, 99.5, 100.0
                writer.writerow(
                    {
                        "timeframe_secs": 3600,
                        "bar": index,
                        "high": high,
                        "low": low,
                        "close": close,
                        "atr": 1.0,
                        "rvol": 1.5,
                        "context_code": context,
                    }
                )
        with (directory / "events.csv").open("w", newline="") as handle:
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
            writer.writerow(
                {
                    "timeframe_secs": 3600,
                    "event_index": 0,
                    "event_type_code": 3,
                    "event_subtype_code": 7,
                    "source_code": 5,
                    "occurred_bar_index": 25,
                    "available_bar_index": 25,
                    "direction_code": 1,
                    "reference_code": "",
                    "primary_value": 101.0,
                    "reference_value": 100.5,
                    "data_completeness": 1.0,
                }
            )

    def test_a_bundle_selects_and_scores_the_event(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            self.write_bundle(run)
            bundle = compression_validation.Bundle("TEST", 2020, run, 4, 2.0, {3600})
            selected = bundle.selected[3600]
            self.assertEqual(len(selected), 1)
            self.assertTrue(bundle.hits[3600][25])
            self.assertTrue(bundle.coiling[3600][24])
            self.assertFalse(bundle.placebo[3600][25])
            result = compression_validation.evaluate(
                [bundle],
                3600,
                4,
                2.0,
                1.96,
                10,
                0,
            )
            self.assertEqual(result["n"], 1)
            self.assertEqual(result["hits"], 1)
            self.assertEqual(result["instruments"]["TEST"]["n"], 1)
            self.assertFalse(result["pass"])

    def test_random_control_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            self.write_bundle(run)
            bundle = compression_validation.Bundle("TEST", 2020, run, 4, 2.0, {3600})
            selected = [
                (0, event) for event in bundle.selected[3600]
            ]
            first = compression_validation.random_control([bundle], 3600, selected, 20, 0)
            second = compression_validation.random_control([bundle], 3600, selected, 20, 0)
            self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()

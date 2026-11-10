#!/usr/bin/env python3
"""Unit tests for the raw forward-edge diagnostic."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

import raw_edge


def bar(
    index: int,
    open_price: float = 100.0,
    high: float = 101.0,
    low: float = 99.0,
    close: float = 100.0,
    atr: float = 1.0,
    ema_stack: int = 1,
    structure_state: int = 1,
) -> raw_edge.Bar:
    return raw_edge.Bar(
        index=index,
        open=open_price,
        high=high,
        low=low,
        close=close,
        atr=atr,
        ema_stack=ema_stack,
        structure_state=structure_state,
    )


class ForwardMetricsTests(unittest.TestCase):
    def test_a_long_target_and_forward_returns(self) -> None:
        bars = (
            bar(0),
            bar(1, high=101.0, low=100.0, close=100.0),
            bar(2, high=104.0, low=100.0, close=103.0),
            bar(3, high=105.0, low=100.0, close=104.0),
        )
        record = raw_edge.forward_metrics(
            bars, 0, "long", (1, 2, 3), 3.0, 1.0, 3, 6.0
        )
        assert record is not None
        self.assertAlmostEqual(record["h1"]["gross_return"], 0.0)
        self.assertAlmostEqual(record["h2"]["gross_return"], 0.03)
        self.assertAlmostEqual(record["h2"]["net_return"], 0.03 - 0.0006)
        self.assertAlmostEqual(record["h2"]["mfe_atr"], 4.0)
        self.assertAlmostEqual(record["h2"]["mae_atr"], 0.0)
        self.assertEqual(record["barrier"]["outcome"], "target")
        self.assertEqual(record["barrier"]["r_multiple"], 3.0)

    def test_a_same_bar_target_and_stop_is_a_stop(self) -> None:
        bars = (
            bar(0),
            bar(1, high=101.0, low=100.0, close=100.0),
            bar(2, high=104.0, low=98.0, close=103.0),
        )
        record = raw_edge.forward_metrics(
            bars, 0, "long", (2,), 3.0, 1.0, 2, 0.0
        )
        assert record is not None
        self.assertEqual(record["barrier"]["outcome"], "stop")
        self.assertEqual(record["barrier"]["r_multiple"], -1.0)

    def test_a_short_target_mirrors_the_long(self) -> None:
        bars = (
            bar(0),
            bar(1, high=100.0, low=99.0, close=100.0),
            bar(2, high=100.0, low=96.0, close=97.0),
        )
        record = raw_edge.forward_metrics(
            bars, 0, "short", (2,), 3.0, 1.0, 2, 0.0
        )
        assert record is not None
        self.assertAlmostEqual(record["h2"]["gross_return"], 0.03)
        self.assertAlmostEqual(record["h2"]["mfe_atr"], 4.0)
        self.assertAlmostEqual(record["h2"]["mae_atr"], 0.0)
        self.assertEqual(record["barrier"]["outcome"], "target")
        self.assertEqual(record["barrier"]["r_multiple"], 3.0)

    def test_a_short_same_bar_target_and_stop_is_a_stop(self) -> None:
        bars = (
            bar(0),
            bar(1, high=100.0, low=99.0, close=100.0),
            bar(2, high=102.0, low=96.0, close=97.0),
        )
        record = raw_edge.forward_metrics(
            bars, 0, "short", (2,), 3.0, 1.0, 2, 0.0
        )
        assert record is not None
        self.assertEqual(record["barrier"]["outcome"], "stop")
        self.assertEqual(record["barrier"]["r_multiple"], -1.0)

    def test_a_missing_forward_bar_is_skipped(self) -> None:
        bars = (bar(0), bar(1))
        record = raw_edge.forward_metrics(
            bars, 0, "long", (1, 2), 3.0, 1.0, 2, 0.0
        )
        assert record is not None
        self.assertIn("h1", record)
        self.assertNotIn("h2", record)

    def test_the_last_bar_has_no_entry(self) -> None:
        bars = (bar(0), bar(1))
        self.assertIsNone(
            raw_edge.forward_metrics(bars, 1, "long", (1,), 3.0, 1.0, 1, 0.0)
        )


class SignalTests(unittest.TestCase):
    def test_the_signal_variants_are_distinct(self) -> None:
        bars = (
            bar(0, ema_stack=1, structure_state=1),
            bar(1, ema_stack=1, structure_state=1),
            bar(2, ema_stack=-1, structure_state=-1),
            bar(3, ema_stack=1, structure_state=1),
        )
        bundle = raw_edge.Bundle(
            label="fixture",
            bars=bars,
            fib={1: frozenset({0}), -1: frozenset({2})},
            ema=frozenset({0, 1, 2, 3}),
        )
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "long", "full"), [0])
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "long", "events"), [0])
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "long", "states"), [0, 1, 3])
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "short", "full"), [2])
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "short", "events"), [2])
        self.assertEqual(raw_edge.signal_bar_indices(bundle, "short", "states"), [2])

    def test_non_overlapping_keeps_the_first_of_each_cluster(self) -> None:
        self.assertEqual(
            raw_edge.non_overlapping([0, 1, 2, 10, 11, 30], 10),
            [0, 10, 30],
        )


class ParsingTests(unittest.TestCase):
    def test_readings_and_events_are_filtered_to_15m(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with (root / "readings.csv").open("w", newline="", encoding="utf-8") as handle:
                writer = csv.writer(handle)
                writer.writerow(
                    [
                        "timeframe_secs",
                        "bar",
                        "open",
                        "high",
                        "low",
                        "close",
                        "atr",
                        "ema_stack",
                        "structure_state",
                    ]
                )
                writer.writerow([900, 0, 100, 101, 99, 100, 1.0, 1, 1])
                writer.writerow([60, 0, 100, 101, 99, 100, 1.0, 1, 1])
            with (root / "events.csv").open("w", newline="", encoding="utf-8") as handle:
                writer = csv.writer(handle)
                writer.writerow(
                    [
                        "timeframe_secs",
                        "event_type_code",
                        "event_subtype_code",
                        "source_code",
                        "available_bar_index",
                        "direction_code",
                    ]
                )
                writer.writerow([900, 1, 1, 56, 0, 1])
                writer.writerow([900, 1, 1, 1, 0, -1])
                writer.writerow([60, 1, 1, 56, 0, 1])
            bundle = raw_edge.load_bundle(root, label="fixture")
            self.assertEqual(len(bundle.bars), 1)
            self.assertEqual(bundle.bars[0].index, 0)
            self.assertEqual(bundle.fib[1], frozenset({0}))
            self.assertEqual(bundle.ema, frozenset({0}))


class SummaryTests(unittest.TestCase):
    def test_summary_counts_and_medians(self) -> None:
        records = [
            {
                "h1": {
                    "gross_return": 0.01,
                    "net_return": 0.0094,
                    "mfe_atr": 1.0,
                    "mae_atr": 0.5,
                },
                "barrier": {"outcome": "target", "r_multiple": 3.0},
            },
            {
                "h1": {
                    "gross_return": -0.01,
                    "net_return": -0.0106,
                    "mfe_atr": 0.2,
                    "mae_atr": 1.2,
                },
                "barrier": {"outcome": "stop", "r_multiple": -1.0},
            },
        ]
        summary = raw_edge.summarize(records, (1,))
        self.assertEqual(summary["count"], 2)
        self.assertEqual(summary["horizons"]["1"]["hit_rate"], 0.5)
        self.assertAlmostEqual(summary["horizons"]["1"]["mean_gross_pct"], 0.0)
        self.assertEqual(summary["barrier"]["target"], 1)
        self.assertEqual(summary["barrier"]["stop"], 1)
        self.assertEqual(summary["barrier"]["mean_r"], 1.0)


if __name__ == "__main__":
    unittest.main()

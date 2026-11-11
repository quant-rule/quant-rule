#!/usr/bin/env python3
"""Fixture tests for the fresh-onset 5m breakout diagnostic.

Run with:

    python3 -m unittest tools/research/test_breakout_edge.py
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from breakout_edge import (  # noqa: E402
    Bar,
    Bundle,
    Onset,
    non_overlapping,
    reconstruct,
    stats,
)


def bar(index: int, open_price: float, high: float, low: float, close: float) -> Bar:
    return Bar(index, open_price, high, low, close, 2)


class ReconstructTests(unittest.TestCase):
    def test_long_break_uses_next_open_and_close(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 111, 112, 110, 111),
            bar(6, 112, 114, 111, 113.5),
        )
        bundle = Bundle("test-2018", bars, (Onset(4, 110, 100),))
        signals = reconstruct(bundle, 3, (1, 3))
        self.assertEqual(len(signals), 1)
        signal = signals[0]
        self.assertEqual(signal.direction, 1)
        self.assertEqual(signal.onset_bar, 4)
        self.assertEqual(signal.break_bar, 5)
        self.assertAlmostEqual(signal.entry, 112.0)
        self.assertAlmostEqual(signal.returns[1], (113.5 - 112.0) / 112.0 * 10_000.0)

    def test_short_break(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 99, 100, 98, 99),
            bar(6, 98, 99, 96, 97),
        )
        bundle = Bundle("test-2018", bars, (Onset(4, 110, 100),))
        signals = reconstruct(bundle, 3, (1,))
        self.assertEqual(len(signals), 1)
        self.assertEqual(signals[0].direction, -1)
        self.assertAlmostEqual(signals[0].returns[1], -(97.0 - 98.0) / 98.0 * 10_000.0)

    def test_boundary_equal_is_not_a_break(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 110, 111, 109, 110),
            bar(6, 110, 112, 109, 111),
            bar(7, 111, 113, 110, 112),
        )
        bundle = Bundle("test-2018", bars, (Onset(4, 110, 100),))
        signals = reconstruct(bundle, 3, (1,))
        self.assertEqual(len(signals), 1)
        self.assertEqual(signals[0].break_bar, 6)
        self.assertEqual(signals[0].direction, 1)

    def test_window_expiry(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 100, 105, 95, 100),
            bar(6, 100, 105, 95, 100),
            bar(7, 100, 105, 95, 100),
            bar(8, 111, 112, 110, 111),
            bar(9, 112, 113, 111, 112),
        )
        bundle = Bundle("test-2018", bars, (Onset(4, 110, 100),))
        self.assertEqual(reconstruct(bundle, 3, (1,)), [])

    def test_next_onset_supersedes(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 100, 105, 95, 100),
            bar(6, 100, 106, 96, 100),
            bar(7, 111, 112, 110, 111),
            bar(8, 112, 113, 111, 112),
        )
        bundle = Bundle(
            "test-2018",
            bars,
            (Onset(4, 110, 100), Onset(6, 106, 96)),
        )
        signals = reconstruct(bundle, 3, (1,))
        self.assertEqual(len(signals), 1)
        self.assertEqual(signals[0].onset_bar, 6)
        self.assertEqual(signals[0].break_bar, 7)

    def test_onset_near_the_end_does_not_panic(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 105, 95, 100),
        )
        bundle = Bundle("test-2018", bars, (Onset(2, 110, 100),))
        self.assertEqual(reconstruct(bundle, 3, (1,)), [])

    def test_gap_open_is_the_entry_price(self) -> None:
        bars = (
            bar(0, 100, 101, 99, 100),
            bar(1, 100, 101, 99, 100),
            bar(2, 100, 101, 99, 100),
            bar(3, 100, 101, 99, 100),
            bar(4, 100, 105, 95, 100),
            bar(5, 111, 112, 110, 111),
            bar(6, 115, 117, 114, 116),
        )
        bundle = Bundle("test-2018", bars, (Onset(4, 110, 100),))
        signal = reconstruct(bundle, 3, (1,))[0]
        self.assertAlmostEqual(signal.entry, 115.0)
        self.assertAlmostEqual(signal.returns[1], (116.0 - 115.0) / 115.0 * 10_000.0)


class SampleTests(unittest.TestCase):
    def test_non_overlapping_keeps_one_per_cluster(self) -> None:
        from breakout_edge import Signal

        direct = [
            Signal("test-2018", 0, break_bar, 1, 100.0, {1: 1.0})
            for break_bar in (10, 15, 31)
        ]
        kept = non_overlapping(direct, 20)
        self.assertEqual([signal.break_bar for signal in kept], [10, 31])

    def test_stats_are_deterministic(self) -> None:
        from breakout_edge import Signal

        signals = [
            Signal("test-2018", index, index, 1, 100.0, {1: value})
            for index, value in enumerate((1.0, 2.0, 3.0))
        ]
        result = stats(signals, 1)
        self.assertEqual(result["count"], 3)
        self.assertAlmostEqual(result["mean_bps"], 2.0)
        self.assertAlmostEqual(result["hit_rate"], 1.0)
        self.assertLess(result["lcb_bps"], result["mean_bps"])


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the barrier path scan."""

from __future__ import annotations

import unittest

import barrier_path


def row(open_price: float, high: float, low: float, close: float, atr: float = 1.0):
    return {
        "open": open_price,
        "high": high,
        "low": low,
        "close": close,
        "atr": atr,
    }


class ScanTests(unittest.TestCase):
    def test_target_before_stop(self) -> None:
        rows = [row(100.0, 101.5, 99.5, 101.0)]
        outcome, r, index = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "target")
        self.assertEqual(r, 1.0)
        self.assertEqual(index, 0)

    def test_stop_before_target_on_the_entry_bar(self) -> None:
        rows = [row(100.0, 100.5, 98.5, 99.0)]
        outcome, r, _ = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "stop")
        self.assertEqual(r, -1.0)

    def test_same_bar_tie_goes_to_the_stop(self) -> None:
        rows = [row(100.0, 102.0, 98.0, 100.0)]
        outcome, _, _ = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "stop")

    def test_gap_at_open_resolves_the_path(self) -> None:
        rows = [
            row(100.0, 100.5, 99.5, 100.0),
            row(102.0, 102.5, 101.5, 102.0),
        ]
        outcome, r, index = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "target")
        self.assertEqual(r, 1.0)
        self.assertEqual(index, 1)

    def test_timeout_marks_the_last_close(self) -> None:
        rows = [row(100.0, 100.2, 99.8, 100.5) for _ in range(5)]
        outcome, r, index = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 4.0, 5)
        self.assertEqual(outcome, "timeout")
        self.assertAlmostEqual(r, 0.5)
        self.assertEqual(index, 4)

    def test_short_target_and_stop_are_symmetric(self) -> None:
        rows = [row(100.0, 100.5, 98.5, 99.0)]
        outcome, r, _ = barrier_path.scan_path(rows, 0, -1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "target")
        self.assertEqual(r, 1.0)

        rows = [row(100.0, 101.5, 99.5, 101.0)]
        outcome, r, _ = barrier_path.scan_path(rows, 0, -1, 100.0, 1.0, 1.0, 20)
        self.assertEqual(outcome, "stop")
        self.assertEqual(r, -1.0)

    def test_a_fade_target_is_not_taken_from_the_pre_entry_low(self) -> None:
        # A fade short enters at the upper boundary; the bar's low happened
        # before the touch, so it cannot be the post-entry target.
        rows = [row(100.0, 105.5, 100.5, 105.0)]
        with_pre_entry = barrier_path.scan_path(
            rows, 0, -1, 105.0, 1.0, 4.0, 20, target_on_entry_bar=False
        )
        self.assertEqual(with_pre_entry[0], "timeout")
        with_pre_entry_target = barrier_path.scan_path(
            rows, 0, -1, 105.0, 1.0, 4.0, 20, target_on_entry_bar=True
        )
        self.assertEqual(with_pre_entry_target[0], "target")

    def test_the_scan_is_deterministic(self) -> None:
        rows = [row(100.0, 101.5, 99.5, 101.0), row(101.0, 102.0, 100.0, 101.5)]
        first = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 2.0, 20)
        second = barrier_path.scan_path(rows, 0, 1, 100.0, 1.0, 2.0, 20)
        self.assertEqual(first, second)


class EntryTests(unittest.TestCase):
    def event(self, upper: float = 105.0, lower: float = 95.0) -> dict:
        return {"available": 0, "primary": upper, "reference": lower}

    def test_a_gap_above_uses_the_open(self) -> None:
        rows = [row(100.0, 100.5, 99.5, 100.0), row(106.0, 107.0, 105.5, 106.0)]
        resolved = barrier_path.entry_for_event(rows, self.event())
        self.assertIsNotNone(resolved)
        index, direction, entry, atr = resolved
        self.assertEqual(index, 1)
        self.assertEqual(direction, 1)
        self.assertEqual(entry, 106.0)
        self.assertEqual(atr, 1.0)

    def test_a_single_touch_uses_the_boundary(self) -> None:
        rows = [row(100.0, 100.5, 99.5, 100.0), row(100.0, 105.5, 99.5, 104.0)]
        resolved = barrier_path.entry_for_event(rows, self.event())
        self.assertIsNotNone(resolved)
        _, direction, entry, _ = resolved
        self.assertEqual(direction, 1)
        self.assertEqual(entry, 105.0)

    def test_a_both_touch_bar_is_skipped(self) -> None:
        rows = [row(100.0, 100.5, 99.5, 100.0), row(100.0, 106.0, 94.0, 100.0)]
        self.assertIsNone(barrier_path.entry_for_event(rows, self.event()))

    def test_no_touch_is_skipped(self) -> None:
        rows = [row(100.0, 100.5, 99.5, 100.0), row(100.0, 101.0, 99.0, 100.0)]
        self.assertIsNone(barrier_path.entry_for_event(rows, self.event()))


if __name__ == "__main__":
    unittest.main()

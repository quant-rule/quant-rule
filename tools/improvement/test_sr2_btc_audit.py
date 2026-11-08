#!/usr/bin/env python3
"""Unit tests for the SR2-A BTCUSD audit helpers."""

from __future__ import annotations

import unittest

from sr2_btc_audit import (
    bootstrap,
    concentration,
    goal_gap,
    path_value,
    percentile,
    t_stat,
)


def bar(index: int, open_price: float, high: float, low: float, close: float) -> dict:
    return {
        "bar": index,
        "timestamp": index * 3_600,
        "open": open_price,
        "high": high,
        "low": low,
        "close": close,
        "atr": 1.0,
        "ema_stack": 0,
        "atr_ratio": 1.0,
        "rvol_ratio": 1.0,
    }


class AuditTests(unittest.TestCase):
    def test_concentration_removes_the_largest_trades(self) -> None:
        result = concentration([5.0, 4.0, 3.0, 2.0, 1.0])
        self.assertEqual(result["total"], 15.0)
        self.assertEqual(result["without_top_1"], 10.0)
        self.assertEqual(result["without_top_3"], 3.0)
        self.assertEqual(result["without_top_5"], 0.0)
        self.assertAlmostEqual(result["top_5_share"], 1.0)

    def test_bootstrap_is_deterministic_and_contains_the_mean(self) -> None:
        first = bootstrap([1.0, 2.0, 3.0, 4.0, 5.0])
        second = bootstrap([1.0, 2.0, 3.0, 4.0, 5.0])
        self.assertEqual(first, second)
        self.assertAlmostEqual(first["mean"], 3.0)
        self.assertLessEqual(first["mean_ci_95"][0], 3.0)
        self.assertGreaterEqual(first["mean_ci_95"][1], 3.0)

    def test_t_stat(self) -> None:
        self.assertGreater(t_stat([1.0, 2.0, 3.0]), 4.0)
        self.assertEqual(t_stat([2.0]), 0.0)

    def test_percentile(self) -> None:
        self.assertEqual(percentile([1.0, 2.0, 3.0, 4.0, 5.0], 0.5), 3.0)
        self.assertEqual(percentile([1.0, 2.0, 3.0, 4.0, 5.0], 0.0), 1.0)
        self.assertEqual(percentile([1.0, 2.0, 3.0, 4.0, 5.0], 1.0), 5.0)

    def test_goal_gap_uses_the_five_cell_denominator(self) -> None:
        result = goal_gap(5_000.0, 0.05)
        self.assertAlmostEqual(result["total_return_5y"], 0.10)
        self.assertAlmostEqual(
            result["monthly_equivalent"],
            (1.10 ** (1.0 / 60.0)) - 1.0,
        )
        self.assertGreater(result["gap_multiple"], 1.0)

    def test_path_stop_and_target(self) -> None:
        rows = [
            bar(0, 100.0, 100.5, 99.5, 100.0),
            bar(1, 100.0, 101.0, 99.8, 100.5),
        ]
        event = {"available": 0, "direction": 1}
        stop = path_value(rows, event, 3.0, 6.0)
        self.assertIsNotNone(stop)
        self.assertEqual(stop["reason"], "stop")
        rows[1]["high"] = 100.2
        rows[1]["low"] = 97.0
        target = path_value(rows, event, 3.0, 6.0)
        self.assertIsNotNone(target)
        self.assertEqual(target["reason"], "target")
        self.assertAlmostEqual(target["value"], 3.0 - 0.06)


if __name__ == "__main__":
    unittest.main()

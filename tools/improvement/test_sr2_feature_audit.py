#!/usr/bin/env python3
"""Unit tests for the SR2 feature attribution and chart helpers."""

from __future__ import annotations

import unittest

from sr2_feature_audit import ema, session, stats, summarize, window_bounds


def row(net: float, *, aligned: bool = False, adx_weak: bool = False, rvol_low: bool = False, cell: str = "btc-2018") -> dict:
    return {
        "net_pnl": net,
        "aligned": aligned,
        "adx_weak": adx_weak,
        "rvol_low": rvol_low,
        "cell": cell,
    }


class FeatureAuditTests(unittest.TestCase):
    def test_ema_has_no_value_before_the_period(self) -> None:
        values = ema([1.0, 2.0, 3.0, 4.0, 5.0], 3)
        self.assertEqual(len(values), 5)
        self.assertIsNone(values[0])
        self.assertIsNone(values[1])
        self.assertIsNotNone(values[2])
        self.assertGreater(values[4], values[2])

    def test_window_is_centred_and_clipped(self) -> None:
        self.assertEqual(window_bounds(50, 100, 20), (40, 60))
        self.assertEqual(window_bounds(10, 100, 20), (0, 20))
        self.assertEqual(window_bounds(99, 100, 20), (80, 100))
        self.assertEqual(window_bounds(2, 10, 20), (0, 10))

    def test_session_buckets(self) -> None:
        self.assertEqual(session(0), "00-06")
        self.assertEqual(session(6), "06-12")
        self.assertEqual(session(12), "12-18")
        self.assertEqual(session(18), "18-24")

    def test_stats(self) -> None:
        result = stats([row(10.0), row(-5.0), row(5.0)])
        self.assertEqual(result["n"], 3)
        self.assertEqual(result["net_pnl"], 10.0)
        self.assertAlmostEqual(result["win_rate"], 2.0 / 3.0)
        self.assertAlmostEqual(result["profit_factor"], 3.0)

    def test_summary_separates_the_conditions(self) -> None:
        rows = [
            row(10.0, aligned=True, adx_weak=True, rvol_low=True),
            row(-5.0, aligned=False, adx_weak=True, rvol_low=False),
            row(5.0, aligned=True, adx_weak=False, rvol_low=True),
        ]
        result = summarize(rows)
        self.assertEqual(result["all"]["n"], 3)
        self.assertEqual(result["aligned"]["n"], 2)
        self.assertEqual(result["rvol_low"]["n"], 2)
        self.assertEqual(result["aligned_adx_weak_rvol_low"]["n"], 1)


if __name__ == "__main__":
    unittest.main()

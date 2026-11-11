#!/usr/bin/env python3
"""Tests for the SIM-03 exit lab."""

from __future__ import annotations

import unittest
from dataclasses import replace
from pathlib import Path

from bundle import Bar, Bundle
from exit_lab import _non_overlapping, simulate


def make_bundle() -> Bundle:
    bars = [
        Bar(
            bar=index,
            timestamp=1514764800 + index * 900,
            open=close,
            high=high,
            low=low,
            close=close,
            volume=100.0,
            atr=1.0,
            ema_fast=ema,
            ema_mid=ema,
            ema_slow=ema,
            rsi=50.0,
            rvol=1.0,
            adx=20.0,
            ema_stack=1,
            rsi_zone=0,
            atr_ratio=1.0,
            raw={"last_pivot_low": "99.5", "last_pivot_high": "103.5"},
        )
        for index, (high, low, close, ema) in enumerate(
            [
                (100.5, 99.5, 100.0, 100.0),
                (102.1, 99.9, 101.0, 100.5),
                (103.0, 101.0, 102.0, 101.0),
                (101.0, 100.0, 100.5, 101.5),
                (102.0, 100.0, 101.0, 101.0),
            ]
        )
    ]
    return Bundle(
        root=Path("."),
        timeframe_secs=900,
        bars=bars,
        events=[],
        levels=[],
        provenance={},
    )


class ExitLabTest(unittest.TestCase):
    def test_fixed_bracket_and_partial(self) -> None:
        result = simulate(make_bundle(), 0, "long", {"horizon_bars": 4, "cost_bps": 6.0})
        self.assertIsNotNone(result)
        self.assertAlmostEqual(result["fixed_2r"]["return_pct"], 0.02 - 0.0006)
        self.assertAlmostEqual(result["partial_1r"]["return_pct"], 0.015 - 0.0006)

    def test_time_stop_is_the_last_close(self) -> None:
        result = simulate(make_bundle(), 0, "long", {"horizon_bars": 4, "cost_bps": 0.0})
        self.assertAlmostEqual(result["time"]["return_pct"], 0.01)

    def test_non_overlapping_keeps_both_sides(self) -> None:
        entries = [
            {"index": 0, "side": "long"},
            {"index": 0, "side": "short"},
            {"index": 3, "side": "long"},
        ]
        selected = _non_overlapping(entries, 5)
        self.assertEqual(len(selected), 2)
        self.assertEqual({row["side"] for row in selected}, {"long", "short"})

    def test_missing_atr_is_skipped(self) -> None:
        bundle = make_bundle()
        bundle.bars[0] = replace(bundle.bars[0], atr=None)
        self.assertIsNone(simulate(bundle, 0, "long", {}))


if __name__ == "__main__":
    unittest.main()

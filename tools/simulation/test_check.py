#!/usr/bin/env python3
"""Tests for the simulation check event study."""

from __future__ import annotations

import unittest
from dataclasses import replace
from datetime import datetime, timedelta, timezone

from bundle import Bar
from check import _non_overlapping, _passes_direction, _passes_filter, check_side


def event_bar(wick: float) -> Bar:
    return Bar(
        bar=0,
        timestamp=1577836800,
        open=100.0,
        high=101.0,
        low=99.0,
        close=100.0,
        volume=100.0,
        atr=1.0,
        ema_fast=100.0,
        ema_mid=100.0,
        ema_slow=100.0,
        rsi=50.0,
        rvol=1.0,
        adx=20.0,
        ema_stack=0,
        rsi_zone=0,
        atr_ratio=1.0,
        raw={"lower_wick_ratio": str(wick), "close_position": "0.5"},
    )


def rows() -> list[dict]:
    start = datetime(2020, 1, 1, tzinfo=timezone.utc)
    result = []
    for index in range(80):
        positive = index % 2 == 0
        result.append(
            {
                "event_bar": event_bar(0.4 if positive else 0.1),
                "entry_index": index * 2,
                "exit_index": index * 2 + 1,
                "entry_time": int((start + timedelta(days=index * 10)).timestamp()),
                "entry_open": 100.0,
                "exit_close": 101.0 if positive else 99.0,
            }
        )
    return result


class CheckTest(unittest.TestCase):
    def test_non_overlapping_keeps_spaced_events(self) -> None:
        selected = _non_overlapping(rows(), 1)
        self.assertEqual(len(selected), 80)

    def test_filtered_set_beats_the_control(self) -> None:
        result = check_side(
            rows(),
            "long",
            {"wick_column": "lower_wick_ratio", "wick_min": 0.3},
            horizon_bars=1,
            cost_bps=0.0,
        )
        self.assertEqual(result["filtered"]["n"], 40)
        self.assertAlmostEqual(result["filtered"]["mean_net"], 0.01)
        self.assertEqual(result["filtered"]["hit_rate"], 1.0)
        self.assertEqual(result["control"]["n"], 80)
        self.assertAlmostEqual(result["control"]["mean_net"], 0.0)
        self.assertEqual(result["verdict"], "survives")

    def test_no_filter_is_the_control(self) -> None:
        result = check_side(
            rows(),
            "long",
            {},
            horizon_bars=1,
            cost_bps=0.0,
        )
        self.assertEqual(result["filtered"]["n"], result["control"]["n"])
        self.assertEqual(result["verdict"], "no_pattern")


    def test_direction_and_close_back_filters(self) -> None:
        base = event_bar(0.2)
        confirmation = replace(
            base,
            bar=1,
            timestamp=base.timestamp + 900,
            high=101.0,
            low=98.5,
            close=99.0,
            raw={"lower_wick_ratio": "0.2", "close_position": "0.5"},
        )
        row = {
            "event_bar": event_bar(0.2),
            "direction": 1,
            "reference": 100.0,
            "confirmation_bar": confirmation,
        }
        self.assertTrue(_passes_direction(row, "long", {"direction_only": True}))
        self.assertFalse(_passes_direction(row, "short", {"direction_only": True}))
        self.assertTrue(_passes_direction(row, "short", {"opposite_direction": True}))
        self.assertFalse(_passes_direction(row, "long", {"opposite_direction": True}))
        self.assertTrue(_passes_filter(row, "short", {"opposite_direction": True, "close_back": True}))
        self.assertTrue(_passes_filter(row, "short", {"opposite_direction": True, "ema_stack": 0}))
        self.assertFalse(_passes_filter(row, "short", {"opposite_direction": True, "ema_stack": -1}))

    def test_cost_is_subtracted(self) -> None:
        result = check_side(
            rows(),
            "long",
            {"wick_column": "lower_wick_ratio", "wick_min": 0.3},
            horizon_bars=1,
            cost_bps=6.0,
        )
        self.assertAlmostEqual(result["filtered"]["mean_net"], 0.01 - 0.0006)


if __name__ == "__main__":
    unittest.main()

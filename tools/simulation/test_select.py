#!/usr/bin/env python3
"""Tests for deterministic simulation window selection."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

from bundle import load_bundle, load_catalog
from window_select import WindowSpec, select_windows

FIXTURE = Path(__file__).parent / "fixtures" / "mini"
REGISTRY = Path(__file__).resolve().parents[2] / "docs" / "registry" / "indicators.json"


def spec(**overrides) -> WindowSpec:
    values = {
        "event_codes": (149,),
        "window_bars": 200,
        "candidate_zone": (60, 100),
        "max_windows": 4,
        "min_windows": 1,
        "seed": 0,
        "overlap_bars": 120,
    }
    values.update(overrides)
    return WindowSpec(**values)


class SelectTest(unittest.TestCase):
    def setUp(self) -> None:
        self.bundle = load_bundle(FIXTURE)
        self.catalog = load_catalog(REGISTRY)

    def test_selects_the_three_fib_bars(self) -> None:
        record = select_windows(self.bundle, spec(), self.catalog)
        self.assertEqual(record["candidate_count"], 3)
        self.assertEqual(record["selected_count"], 3)
        self.assertFalse(record["underpowered"])
        bars = [window["candidate_bar"] for window in record["windows"]]
        self.assertEqual(bars, [60, 160, 260])
        for window in record["windows"]:
            self.assertEqual(window["events"][0]["name"], "level.fib_test")

    def test_same_seed_is_identical(self) -> None:
        first = select_windows(self.bundle, spec(), self.catalog)
        second = select_windows(self.bundle, spec(), self.catalog)
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))

    def test_max_windows_is_respected(self) -> None:
        record = select_windows(self.bundle, spec(max_windows=2, min_windows=1), self.catalog)
        self.assertEqual(record["selected_count"], 2)

    def test_underpowered_flag(self) -> None:
        record = select_windows(self.bundle, spec(min_windows=10), self.catalog)
        self.assertTrue(record["underpowered"])

    def test_date_filter_reduces_candidates(self) -> None:
        from_ts = 1514764800 + 900 * 100
        record = select_windows(
            self.bundle, spec(from_ts=from_ts, max_windows=4), self.catalog
        )
        self.assertEqual(record["candidate_count"], 2)
        self.assertEqual([window["candidate_bar"] for window in record["windows"]], [160, 260])

    def test_window_bounds_are_inside_the_series(self) -> None:
        record = select_windows(self.bundle, spec(), self.catalog)
        for window in record["windows"]:
            self.assertGreaterEqual(window["start_index"], 0)
            self.assertLessEqual(window["end_index"], len(self.bundle.bars))
            self.assertGreaterEqual(window["candidate_index"], window["start_index"])
            self.assertLess(window["candidate_index"], window["end_index"])

    def test_empty_event_codes_are_rejected(self) -> None:
        with self.assertRaises(Exception):
            select_windows(self.bundle, spec(event_codes=()), self.catalog)


if __name__ == "__main__":
    unittest.main()

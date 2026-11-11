#!/usr/bin/env python3
"""Tests for the deterministic hindsight oracle."""

from __future__ import annotations

import unittest
from pathlib import Path

from bundle import Bar, Bundle
from oracle import OracleSpec, oracle_for_window
from window_select import Window


def make_bundle(highs: list[float], lows: list[float], closes: list[float]) -> Bundle:
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
            ema_fast=close,
            ema_mid=close,
            ema_slow=close,
            rsi=50.0,
            rvol=1.0,
            adx=20.0,
            ema_stack=1,
            rsi_zone=0,
            atr_ratio=1.0,
            raw={},
        )
        for index, (high, low, close) in enumerate(zip(highs, lows, closes))
    ]
    return Bundle(
        root=Path("."),
        timeframe_secs=900,
        bars=bars,
        events=[],
        levels=[],
        provenance={},
    )


def window(bundle: Bundle) -> Window:
    return Window(
        window_id="w001",
        start_index=0,
        end_index=len(bundle.bars),
        candidate_index=0,
        events=(),
    )


def marks_for(bundle: Bundle, side: str, quality: str):
    marks, details = oracle_for_window(
        bundle,
        window(bundle),
        "sim-w001.png",
        OracleSpec(entry_offset_max=0),
        "SIM-T",
    )
    entries = [
        mark
        for mark in marks
        if mark["side"] == side and mark["quality"] == quality and mark["action"] == "entry"
    ]
    exits = [
        mark
        for mark in marks
        if mark["side"] == side and mark["quality"] == quality and mark["action"] == "exit"
    ]
    return entries, exits, details


class OracleTest(unittest.TestCase):
    def test_long_target_first_is_good(self) -> None:
        bundle = make_bundle(
            highs=[100.0, 102.1, 102.5],
            lows=[99.5, 99.9, 99.8],
            closes=[100.0, 100.0, 100.0],
        )
        entries, exits, details = marks_for(bundle, "long", "good")
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0]["bar"], "0")
        self.assertEqual(len(exits), 1)
        self.assertEqual(exits[0]["bar"], "1")
        detail = next(row for row in details if row["side"] == "long" and row["quality"] == "good")
        self.assertAlmostEqual(detail["mfe_atr"], 2.5)
        self.assertEqual(detail["perfect_exit_bar"], 2)

    def test_long_stop_first_is_bad(self) -> None:
        bundle = make_bundle(
            highs=[100.0, 100.2, 100.4],
            lows=[99.5, 98.4, 98.9],
            closes=[100.0, 100.0, 100.0],
        )
        entries, exits, details = marks_for(bundle, "long", "bad")
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0]["bar"], "0")
        self.assertEqual(len(exits), 1)
        self.assertEqual(exits[0]["bar"], "1")
        detail = next(row for row in details if row["side"] == "long" and row["quality"] == "bad")
        self.assertTrue(detail["bad"])
        self.assertAlmostEqual(detail["mae_atr"], 1.6)

    def test_short_target_first_is_good(self) -> None:
        bundle = make_bundle(
            highs=[100.0, 99.9, 99.8],
            lows=[99.5, 97.9, 97.5],
            closes=[100.0, 100.0, 100.0],
        )
        entries, exits, _ = marks_for(bundle, "short", "good")
        self.assertEqual(len(entries), 1)
        self.assertEqual(len(exits), 1)
        self.assertEqual(entries[0]["bar"], "0")
        self.assertEqual(exits[0]["bar"], "1")

    def test_short_stop_first_is_bad(self) -> None:
        bundle = make_bundle(
            highs=[100.0, 101.8, 102.0],
            lows=[99.5, 99.8, 99.0],
            closes=[100.0, 100.0, 100.0],
        )
        entries, exits, details = marks_for(bundle, "short", "bad")
        self.assertEqual(len(entries), 1)
        self.assertEqual(len(exits), 1)
        detail = next(row for row in details if row["side"] == "short" and row["quality"] == "bad")
        self.assertTrue(detail["bad"])

    def test_entry_offset_can_wait_for_confirmation(self) -> None:
        bundle = make_bundle(
            highs=[100.0, 100.0, 102.1],
            lows=[99.5, 99.5, 99.9],
            closes=[100.0, 100.0, 100.0],
        )
        marks, _ = oracle_for_window(
            bundle,
            window(bundle),
            "sim-w001.png",
            OracleSpec(entry_offset_max=1),
            "SIM-T",
        )
        entries = [mark for mark in marks if mark["action"] == "entry" and mark["side"] == "long"]
        self.assertTrue(entries)
        self.assertEqual(entries[0]["bar"], "1")


if __name__ == "__main__":
    unittest.main()

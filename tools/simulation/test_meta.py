#!/usr/bin/env python3
"""Tests for the SIM-01 meta-label helpers."""

from __future__ import annotations

import unittest
from pathlib import Path

from bundle import Bar, Bundle, utc_from_seconds
from meta import atr_bucket, best_cell, cell_table, forward_50, meta_rows


def make_bundle() -> Bundle:
    bars = []
    for index in range(80):
        close = 100.0 + index * 0.5
        bars.append(
            Bar(
                bar=index,
                timestamp=1514764800 + index * 900,
                open=close - 0.1,
                high=close + 0.6,
                low=close - 0.6,
                close=close,
                volume=100.0,
                atr=1.0,
                ema_fast=close,
                ema_mid=close,
                ema_slow=close,
                rsi=55.0,
                rvol=1.0,
                adx=25.0,
                ema_stack=1,
                rsi_zone=0,
                atr_ratio=1.1,
                raw={
                    "close_position": "0.6",
                    "upper_wick_ratio": "0.2",
                    "lower_wick_ratio": "0.3",
                    "body_ratio": "0.5",
                },
            )
        )
    return Bundle(
        root=Path("."),
        timeframe_secs=900,
        bars=bars,
        events=[],
        levels=[],
        provenance={},
    )


def mark() -> dict[str, str]:
    return {
        "chart": "human.png",
        "bar": "10",
        "marked_time_utc": utc_from_seconds(1514764800 + 10 * 900),
        "action": "entry",
        "quality": "good",
        "side": "long",
        "note": "",
    }


class MetaTest(unittest.TestCase):
    def test_forward_path_is_positive(self) -> None:
        path = forward_50(make_bundle(), 10, "long", 50)
        self.assertIsNotNone(path)
        self.assertGreater(path["return_50"], 0)
        self.assertGreater(path["mfe_50"], 0)

    def test_meta_labels_the_human_mark(self) -> None:
        rows, unmatched = meta_rows(make_bundle(), [mark()], {}, 50)
        self.assertEqual(unmatched, 0)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["quality"], "good")
        self.assertEqual(rows[0]["ema_stack"], "bull")

    def test_atr_bucket(self) -> None:
        self.assertEqual(atr_bucket(1.0), "ge1")
        self.assertEqual(atr_bucket(0.99), "lt1")
        self.assertEqual(atr_bucket(None), "unknown")

    def test_best_cell_uses_the_minimum(self) -> None:
        rows = [
            {"side": "long", "ema_stack": "bull", "atr_bucket": "ge1", "return_50": 0.01, "mfe_50": 2.0, "mae_50": 0.5},
            {"side": "long", "ema_stack": "bull", "atr_bucket": "ge1", "return_50": 0.02, "mfe_50": 2.0, "mae_50": 0.5},
            {"side": "long", "ema_stack": "bear", "atr_bucket": "lt1", "return_50": -0.01, "mfe_50": 1.0, "mae_50": 1.0},
        ]
        table = cell_table(rows)
        self.assertIsNone(best_cell(table, 3))
        self.assertIsNotNone(best_cell(table, 2))


if __name__ == "__main__":
    unittest.main()

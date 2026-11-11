#!/usr/bin/env python3
"""Tests for the simulation review table."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from bundle import Bar, Bundle
from review import _pairs, _trade_stats, build_review


def make_bundle() -> Bundle:
    bars = [
        Bar(
            bar=index,
            timestamp=1514764800 + index * 900,
            open=close,
            high=close + 0.5,
            low=close - 0.5,
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
        for index, close in enumerate([100.0, 101.0])
    ]
    return Bundle(
        root=Path("."),
        timeframe_secs=900,
        bars=bars,
        events=[],
        levels=[],
        provenance={},
    )


def marks() -> list[dict[str, str]]:
    return [
        {
            "chart": "sim-w001.png",
            "bar": "0",
            "marked_time_utc": "2018-01-01T00:00:00Z",
            "action": "entry",
            "quality": "good",
            "side": "long",
            "note": "",
        },
        {
            "chart": "sim-w001.png",
            "bar": "1",
            "marked_time_utc": "2018-01-01T00:15:00Z",
            "action": "exit",
            "quality": "good",
            "side": "long",
            "note": "",
        },
    ]


class ReviewTest(unittest.TestCase):
    def test_pairs_and_return(self) -> None:
        pairs = _pairs(marks())
        self.assertEqual(len(pairs), 1)
        stats = _trade_stats(make_bundle(), pairs[0])
        self.assertAlmostEqual(stats["return_pct"], 0.01)

    def test_build_review_writes_tables(self) -> None:
        details = [
            {
                "chart": "sim-w001.png",
                "side": "long",
                "quality": "good",
                "mfe_atr": 2.0,
                "mae_atr": 0.5,
            }
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "review.md"
            build_review(
                make_bundle(),
                [{"file": "sim-w001.png"}],
                marks(),
                details,
                path,
                "SIM-T",
                "test premise",
                {},
            )
            text = path.read_text(encoding="utf-8")
        self.assertIn("Review — SIM-T", text)
        self.assertIn("| good | long |", text)
        self.assertIn("test premise", text)


if __name__ == "__main__":
    unittest.main()

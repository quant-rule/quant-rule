#!/usr/bin/env python3
"""Tests for the annotation zone extractor."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import annotation_zones


def row(
    bar: int,
    close: float = 100.0,
    high: float = 101.0,
    low: float = 99.0,
    atr: float = 1.0,
    ema_fast: float = 100.0,
    ema_mid: float = 99.0,
    ema_slow: float = 98.0,
    fib_nearest_atr: float = 0.5,
    band_lower: float = 98.0,
    band_upper: float = 102.0,
    band_center: float = 100.0,
) -> dict[str, str]:
    return {
        "bar": str(bar),
        "close": str(close),
        "high": str(high),
        "low": str(low),
        "atr": str(atr),
        "ema_fast": str(ema_fast),
        "ema_mid": str(ema_mid),
        "ema_slow": str(ema_slow),
        "rsi": "50",
        "adx": "25",
        "ema_stack": "1",
        "structure_state": "1",
        "macd_state": "1",
        "rsi_zone": "0",
        "vwap_distance_atr": "0.1",
        "poc_distance_atr": "0.2",
        "va_position": "0.5",
        "range_position": "0.5",
        "close_position": "0.5",
        "lower_high_count": "1",
        "lower_low_count": "0",
        "higher_high_count": "1",
        "higher_low_count": "0",
        "fib_nearest_atr": str(fib_nearest_atr),
        "fib_nearest_ratio": "0.5",
        "fib_position": "0.5",
        "band_touches": "2",
        "band_width_atr": "0.4",
        "band_position": "0.5",
        "band_lower": str(band_lower),
        "band_upper": str(band_upper),
        "band_center": str(band_center),
    }


def write_rows(path: Path, rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


class ZoneTest(unittest.TestCase):
    def test_zone_includes_before_mark_and_after_mark(self) -> None:
        context = [row(bar) for bar in range(1, 6)]
        post = [row(bar) for bar in range(6, 9)]
        zone = annotation_zones.zone_rows(context, post, 2)
        self.assertEqual([item["bar"] for item in zone], ["3", "4", "5", "6", "7"])

    def test_ema_tap_uses_high_or_low_within_tolerance(self) -> None:
        self.assertTrue(
            annotation_zones.ema_tap(
                [row(1, high=101.0, low=99.9, ema_fast=100.0)], "ema_fast", 0.2
            )
        )
        self.assertFalse(
            annotation_zones.ema_tap(
                [row(1, high=103.0, low=102.0, ema_fast=100.0)], "ema_fast", 0.2
            )
        )

    def test_fib_and_band_tests_use_the_zone(self) -> None:
        self.assertTrue(annotation_zones.fib_test([row(1, fib_nearest_atr=0.1)], 0.2))
        self.assertFalse(annotation_zones.fib_test([row(1, fib_nearest_atr=0.3)], 0.2))
        self.assertTrue(annotation_zones.band_test([row(1, close=100.0)]))
        self.assertFalse(annotation_zones.band_test([row(1, close=103.0)]))

    def test_forward_returns_are_side_adjusted(self) -> None:
        mark = row(5, close=100.0)
        post = [row(bar, close=100.0 + bar, high=100.0 + bar, low=99.0 + bar) for bar in range(1, 6)]
        long = annotation_zones.forward(mark, post, "long")
        short = annotation_zones.forward(mark, post, "short")
        self.assertAlmostEqual(long["fwd_4"], 0.04)
        self.assertAlmostEqual(short["fwd_4"], -0.04)
        self.assertAlmostEqual(long["mfe_50"], 0.05)
        self.assertAlmostEqual(long["mae_50"], 0.0)

    def test_annotation_row_and_summary_are_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            context = [row(bar) for bar in range(1, 6)]
            post = [row(bar, close=100.0 + (bar - 5)) for bar in range(6, 11)]
            write_rows(root / "annotation-001" / "context.csv", context)
            write_rows(root / "annotation-001" / "post.csv", post)
            index = {
                "annotations": [
                    {
                        "annotation": {
                            "action": "entry",
                            "bar": "5",
                            "chart": "chart.png",
                            "marked_time_utc": "2024-01-01T00:00:00Z",
                            "note": "",
                            "quality": "good",
                            "side": "long",
                        },
                        "bar": 5,
                        "time": "2024-01-01 00:00:00+00:00",
                        "row": 1,
                        "path": str(root / "annotation-001"),
                    }
                ]
            }
            (root / "index.json").write_text(json.dumps(index), encoding="utf-8")
            result = annotation_zones.annotation_row(
                root, index["annotations"][0], radius=2, ema_atr=0.2, fib_atr=0.2
            )
            self.assertEqual(result["year"], "2024")
            self.assertTrue(result["ema_tap_any"])
            summary = annotation_zones.summarize([result, dict(result, year="2025")])
            self.assertEqual(summary["count"], 2)
            self.assertEqual(summary["groups"]["all|long|entry"]["count"], 2)
            self.assertAlmostEqual(summary["groups"]["all|long|entry"]["fwd_4_median"], 0.04)


if __name__ == "__main__":
    unittest.main()

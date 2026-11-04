#!/usr/bin/env python3
"""Unit tests for the V5 Phase 2 spike."""

from __future__ import annotations

import unittest

import v5_phase2_spike as spike


def bundle(rows, levels=None, instrument="BTCUSD", fold=2018):
    return spike.Bundle(
        instrument=instrument,
        fold=fold,
        rows=rows,
        by_timestamp={row["timestamp"]: index for index, row in enumerate(rows)},
        levels=levels or [],
        hits={-1: [None] * len(rows), 1: [None] * len(rows)},
        valid={-1: [], 1: []},
        base={-1: None, 1: None},
    )


def bar(timestamp, open_price, high, low, close, atr=1.0, volume=1.0):
    return {
        "timestamp": timestamp,
        "open": open_price,
        "high": high,
        "low": low,
        "close": close,
        "volume": volume,
        "atr": atr,
    }


class MomentumTests(unittest.TestCase):
    def test_momentum_bar_closes_at_extreme(self) -> None:
        rows = [
            bar(0, 100.0, 101.0, 99.0, 100.0),
            bar(3600, 100.0, 102.0, 100.0, 101.9),
        ]
        self.assertEqual(spike.momentum_events(bundle(rows)), [(1, 1)])

    def test_momentum_bar_requires_wide_range(self) -> None:
        rows = [bar(0, 100.0, 100.8, 100.0, 100.7)]
        self.assertEqual(spike.momentum_events(bundle(rows)), [])


class RejectionTests(unittest.TestCase):
    def test_rejection_at_confirmed_level(self) -> None:
        rows = [bar(0, 100.0, 101.5, 99.5, 99.8)]
        with_level = bundle(rows, levels=[(0, 101.4)])
        self.assertEqual(spike.rejection_events(with_level), [(0, -1)])
        without = bundle(rows, levels=[])
        self.assertEqual(spike.rejection_events(without), [])

    def test_level_must_be_available(self) -> None:
        rows = [bar(0, 100.0, 101.5, 99.5, 99.8)]
        future_level = bundle(rows, levels=[(1, 101.4)])
        self.assertEqual(spike.rejection_events(future_level), [])
        stale_level = bundle(rows, levels=[(0, 101.4)])
        stale_level.levels = [(-spike.LEVEL_LOOKBACK - 1, 101.4)]
        self.assertEqual(spike.rejection_events(stale_level), [])


class BlockTests(unittest.TestCase):
    def test_fixed_block_clusters(self) -> None:
        entries = [
            (0, 0, 1, True),
            (0, 50, 1, False),
            (0, 150, 1, True),
        ]
        stats = spike.block_stats(entries)
        self.assertEqual(stats["n"], 3)
        self.assertEqual(stats["blocks"], 2)

    def test_empty_entries(self) -> None:
        stats = spike.block_stats([])
        self.assertEqual(stats["n"], 0)
        self.assertEqual(stats["blocks"], 0)
        self.assertIsNone(stats["hit_rate"])


class RelativeStrengthTests(unittest.TestCase):
    def test_relative_strength_cross(self) -> None:
        timestamps = [index * 3600 for index in range(26)]
        rows = []
        for index, timestamp in enumerate(timestamps):
            close = 100.0
            rows.append(bar(timestamp, close, close, close, close))
        rows[-1]["close"] = 105.0
        rows[-1]["high"] = 105.0
        bundles = [
            bundle(rows, instrument="BTCUSD"),
            bundle(
                [bar(timestamp, 100.0, 100.0, 100.0, 100.0) for timestamp in timestamps],
                instrument="ETHUSD",
            ),
            bundle(
                [bar(timestamp, 100.0, 100.0, 100.0, 100.0) for timestamp in timestamps],
                instrument="XAUUSD",
            ),
        ]
        events = spike.relative_strength_events(bundles)
        self.assertEqual(events[("BTCUSD", 2018)], [(25, 1)])


if __name__ == "__main__":
    unittest.main()

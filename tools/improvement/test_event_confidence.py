"""Deterministic tests for event-confidence joining and grouping."""

from __future__ import annotations

import unittest

import event_confidence as events


class EventConfidenceMath(unittest.TestCase):
    def test_available_bar_is_used_not_occurred_bar(self) -> None:
        raw = [
            {
                "timeframe": "15m",
                "direction": 1,
                "available_bar": 1,
                "occurred_bar": 0,
                "type_label": "level",
                "subtype_label": "break",
            }
        ]
        closes = {0: 100.0, 1: 101.0, 2: 102.0}
        rows = events.event_rows(raw, closes, "15m", (1,))
        self.assertEqual(len(rows), 1)
        self.assertAlmostEqual(rows[0]["forward"], 102.0 / 101.0 - 1.0)
        self.assertTrue(rows[0]["hit"])

    def test_events_from_another_timeframe_are_skipped(self) -> None:
        raw = [
            {
                "timeframe": "1h",
                "direction": 1,
                "available_bar": 0,
                "type_label": "level",
                "subtype_label": "break",
            }
        ]
        self.assertEqual(events.event_rows(raw, {0: 100.0, 1: 101.0}, "15m", (1,)), [])

    def test_grouping_counts_hits(self) -> None:
        rows = [
            {"type": "level", "subtype": "break", "horizon": 1, "hit": True},
            {"type": "level", "subtype": "break", "horizon": 1, "hit": False},
        ]
        grouped = events.grouped(rows, ("type",))
        self.assertEqual(grouped[0]["evaluated"], 2)
        self.assertEqual(grouped[0]["hits"], 1)
        self.assertEqual(grouped[0]["hit_rate"], 0.5)


if __name__ == "__main__":
    unittest.main()

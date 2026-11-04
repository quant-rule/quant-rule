#!/usr/bin/env python3
"""Unit tests for the EURUSD DST-aware session spike."""

from __future__ import annotations

import unittest
from datetime import datetime, timezone

import eurusd_session_spike as spike


def stamp(year: int, month: int, day: int, hour: int, minute: int = 0) -> int:
    return int(datetime(year, month, day, hour, minute, tzinfo=timezone.utc).timestamp())


class CalendarTests(unittest.TestCase):
    def test_london_dst(self) -> None:
        self.assertTrue(spike.in_london(stamp(2018, 1, 15, 8, 0)))
        self.assertFalse(spike.in_london(stamp(2018, 1, 15, 7, 0)))
        self.assertTrue(spike.in_london(stamp(2018, 7, 16, 7, 0)))
        self.assertFalse(spike.in_london(stamp(2018, 7, 16, 16, 30)))

    def test_new_york_dst(self) -> None:
        self.assertTrue(spike.in_new_york(stamp(2018, 1, 15, 14, 30)))
        self.assertTrue(spike.in_new_york(stamp(2018, 7, 16, 13, 30)))
        self.assertFalse(spike.in_new_york(stamp(2018, 7, 16, 20, 0)))


class EventTests(unittest.TestCase):
    def test_london_orb_uses_completed_range_and_next_open(self) -> None:
        first = stamp(2018, 1, 15, 8, 0)
        rows = []
        for index in range(5):
            rows.append(
                {
                    "timestamp": first + index * 900,
                    "open": 1.1000,
                    "high": 1.1005 if index < 2 else 1.1010,
                    "low": 1.0995,
                    "close": 1.1000 if index < 2 else 1.1010,
                    "volume": 1.0,
                    "atr": 0.0010,
                }
            )
        by_timestamp = {row["timestamp"]: index for index, row in enumerate(rows)}
        bundle = spike.Bundle(
            fold=2018,
            rows=rows,
            hits={1: [True] * len(rows), -1: [True] * len(rows)},
            by_timestamp=by_timestamp,
            session_bars={"london": [0, 1, 2, 3, 4], "newyork": []},
            close_bars=[],
        )
        events = spike.orb_events(bundle, "london", spike.LONDON)
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["direction"], 1)
        self.assertEqual(events[0]["event_bar"], 2)
        self.assertEqual(events[0]["entry"], 3)


if __name__ == "__main__":
    unittest.main()

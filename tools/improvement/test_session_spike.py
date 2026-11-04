#!/usr/bin/env python3
"""Unit tests for the session/liquidity spike."""

from __future__ import annotations

import unittest

import session_spike as spike

DAY = 86_400


def bundle() -> spike.Bundle:
    rows = []
    for index, second in enumerate((7, 7.25, 7.5, 7.75, 8)):
        timestamp = DAY + int(second * 3600)
        rows.append(
            {
                "bar": index,
                "timestamp": timestamp,
                "open": 100.0,
                "high": 101.0 if index < 2 else 102.0,
                "low": 99.0,
                "close": 101.0 if index < 2 else 101.5,
                "volume": 1.0,
                "atr": 1.0,
            }
        )
    by_timestamp = {row["timestamp"]: index for index, row in enumerate(rows)}
    hits = {1: [True] * len(rows), -1: [True] * len(rows)}
    return spike.Bundle(
        instrument="XAUUSD",
        fold=2018,
        rows=rows,
        hits=hits,
        by_timestamp=by_timestamp,
        session_bars={"london": [0, 1, 2, 3, 4], "newyork": []},
        close_bars=[],
    )


class SessionTests(unittest.TestCase):
    def test_session_mapping(self) -> None:
        self.assertEqual(spike.session_name(7 * 3600), "london")
        self.assertEqual(spike.session_name(13 * 3600 + 1800), "newyork")
        self.assertIsNone(spike.session_name(21 * 3600))
        self.assertIsNone(spike.session_name(0))

    def test_orb_uses_completed_opening_range_and_next_open(self) -> None:
        events = spike.orb_events(bundle(), "london")
        self.assertEqual(len(events), 1)
        event = events[0]
        self.assertEqual(event["direction"], 1)
        self.assertEqual(event["event_bar"], 2)
        self.assertEqual(event["entry"], 3)


if __name__ == "__main__":
    unittest.main()

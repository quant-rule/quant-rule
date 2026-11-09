#!/usr/bin/env python3
"""Unit tests for the H3 count gate."""

from __future__ import annotations

import unittest

from h3_count import is_h3, is_target_event, side_of


class H3CountTests(unittest.TestCase):
    def test_side_is_opposite_the_event(self) -> None:
        self.assertEqual(side_of(-1), "long")
        self.assertEqual(side_of(1), "short")
        self.assertIsNone(side_of(0))

    def test_only_the_compression_breakout_code_is_counted(self) -> None:
        base = {
            "timeframe_secs": "3600",
            "event_code": "17",
            "event_type_code": "3",
            "event_subtype_code": "7",
            "source_code": "5",
        }
        self.assertTrue(is_target_event(base))
        self.assertFalse(is_target_event({**base, "event_code": "18"}))
        self.assertFalse(is_target_event({**base, "timeframe_secs": "900"}))

    def test_h3_requires_stack_alignment_and_weak_adx(self) -> None:
        self.assertTrue(is_h3(-1, -1, 0))
        self.assertFalse(is_h3(-1, -1, 1))
        self.assertFalse(is_h3(-1, 0, 0))


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the absorption count gate."""

from __future__ import annotations

import unittest

from absorption_count import as_of, gate_report, merge_counts, side_of


class AbsorptionCountTests(unittest.TestCase):
    def test_as_of_uses_the_latest_closed_context_bar(self) -> None:
        closes = [3600, 7200, 10800]
        self.assertEqual(as_of(closes, 9000), 1)
        self.assertEqual(as_of(closes, 10800), 2)
        self.assertIsNone(as_of(closes, 3599))

    def test_side_mapping(self) -> None:
        self.assertEqual(side_of(1), "long")
        self.assertEqual(side_of(-1), "short")
        self.assertIsNone(side_of(0))

    def test_gate_requires_both_sides(self) -> None:
        count = {"totals": {"a1": {"long": 30, "short": 29}}}
        self.assertFalse(gate_report(count, 30)["pass"])
        count["totals"]["a1"]["short"] = 30
        self.assertTrue(gate_report(count, 30)["pass"])

    def test_merge_adds_years_and_sides(self) -> None:
        def bundle(year: str, long: int, short: int) -> dict:
            return {
                "totals": {name: {"long": long, "short": short} for name in ("all", "a1", "a2", "a3")},
                "by_year": {
                    year: {name: {"long": long, "short": short} for name in ("all", "a1", "a2", "a3")}
                },
                "skipped": {"missing_bar": 0, "missing_context": 0, "zero_direction": 0},
                "events": long + short,
            }

        merged = merge_counts([bundle("2018", 10, 5), bundle("2019", 20, 25)])
        self.assertEqual(merged["totals"]["a1"], {"long": 30, "short": 30})
        self.assertEqual(merged["events"], 60)
        self.assertEqual(merged["by_year"]["2019"]["a1"], {"long": 20, "short": 25})


if __name__ == "__main__":
    unittest.main()

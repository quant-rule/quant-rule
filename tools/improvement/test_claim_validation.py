#!/usr/bin/env python3
"""Unit tests for the claim-validation and gate-comparison tools."""

from __future__ import annotations

import unittest

import claim_validation
import gate_compare


class LowerBoundTests(unittest.TestCase):
    def test_more_samples_tighten_the_bound(self) -> None:
        small = claim_validation.lower_bound(55, 100, 1.96)
        large = claim_validation.lower_bound(550, 1000, 1.96)
        self.assertIsNotNone(small)
        self.assertIsNotNone(large)
        self.assertGreater(large, small)

    def test_no_samples_is_none(self) -> None:
        self.assertIsNone(claim_validation.lower_bound(0, 0, 1.96))


class GradeTests(unittest.TestCase):
    def cell(self, hits: int, n: int, base_hits: int, base_n: int, instruments: dict) -> dict:
        return {
            "hits": hits,
            "n": n,
            "available": n,
            "base_positive": base_hits,
            "base_total": base_n,
            "instruments": instruments,
        }

    def test_a_strong_replicated_cell_is_a(self) -> None:
        cell = self.cell(
            600,
            1000,
            500,
            1000,
            {name: {"n": 200, "hits": 120} for name in ("BTC", "ETH", "XAU")},
        )
        self.assertEqual(claim_validation.grade(cell, 2.5), "A")

    def test_a_thin_edge_is_c(self) -> None:
        cell = self.cell(
            510,
            1000,
            500,
            1000,
            {name: {"n": 200, "hits": 102} for name in ("BTC", "ETH", "XAU")},
        )
        self.assertEqual(claim_validation.grade(cell, 2.5), "C")

    def test_low_coverage_is_d(self) -> None:
        cell = self.cell(60, 100, 50, 100, {"BTC": {"n": 100, "hits": 60}})
        cell["available"] = 200
        self.assertEqual(claim_validation.grade(cell, 2.5), "D")


class GateComparisonTests(unittest.TestCase):
    def document(self, grade: str) -> dict:
        return {
            "gate": "pass" if grade in ("A", "B") else "no_edge",
            "primary": [
                {
                    "family": "divergence.regular_bullish",
                    "timeframe": "5m",
                    "grade": grade,
                    "hit_rate": 0.5,
                    "lcb_corrected": 0.49,
                    "lift": 0.0,
                }
            ],
        }

    def test_validation_only_a_cell_is_not_a_pass(self) -> None:
        result = gate_compare.compare(self.document("C"), self.document("B"))
        self.assertEqual(result["gate"], "no_edge")
        self.assertEqual(result["validation_only"], ["divergence.regular_bullish 5m"])

    def test_a_cell_on_both_partitions_passes(self) -> None:
        result = gate_compare.compare(self.document("B"), self.document("B"))
        self.assertEqual(result["gate"], "pass")
        self.assertEqual(result["passing"], ["divergence.regular_bullish 5m"])


if __name__ == "__main__":
    unittest.main()

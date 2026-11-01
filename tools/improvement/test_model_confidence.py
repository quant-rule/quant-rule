"""Deterministic tests for the model-confidence evidence math."""

from __future__ import annotations

import unittest

import model_confidence as confidence


def item(series_index: int, close: float, direction: float | None) -> dict:
    return {
        "series_index": series_index,
        "close": close,
        "record": {
            "series_index": series_index,
            "close": close,
            "direction": {"value": direction},
            "momentum": {"force": direction},
            "structure": {"score": direction, "code": 1 if (direction or 0) > 0 else -1},
            "progression": {"progress": direction, "stage": 2},
        },
    }


class ModelConfidenceMath(unittest.TestCase):
    def test_horizons_are_sorted_and_deduplicated(self) -> None:
        self.assertEqual(confidence.parse_horizons("5,1,3,5"), (1, 3, 5))

    def test_horizons_reject_zero_and_text(self) -> None:
        with self.assertRaises(confidence.ToolError):
            confidence.parse_horizons("0,5")
        with self.assertRaises(confidence.ToolError):
            confidence.parse_horizons("fast")

    def test_average_ranks_share_ties(self) -> None:
        self.assertEqual(
            confidence.average_ranks([10.0, 20.0, 20.0, 30.0]),
            [1.0, 2.5, 2.5, 4.0],
        )

    def test_spearman_is_signed(self) -> None:
        self.assertAlmostEqual(
            confidence.spearman([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), 1.0
        )
        self.assertAlmostEqual(
            confidence.spearman([1.0, 2.0, 3.0], [3.0, 2.0, 1.0]), -1.0
        )

    def test_bucket_monotonicity(self) -> None:
        rows = [
            {"code": 1, "mean_forward_return": 0.01},
            {"code": 2, "mean_forward_return": 0.02},
            {"code": 3, "mean_forward_return": 0.03},
        ]
        correlation, up, down = confidence.bucket_monotonicity(rows, "code")
        self.assertAlmostEqual(correlation, 1.0)
        self.assertTrue(up)
        self.assertFalse(down)

    def test_regime_buckets_are_pre_registered(self) -> None:
        self.assertEqual(confidence.regime_bucket("atr_ratio", 0.7), "atr<0.8")
        self.assertEqual(confidence.regime_bucket("atr_ratio", 0.8), "atr0.8-1.2")
        self.assertEqual(confidence.regime_bucket("atr_ratio", 1.3), "atr>1.2")
        self.assertEqual(confidence.regime_bucket("rvol_ratio", 0.5), "rvol<1.0")
        self.assertEqual(confidence.regime_bucket("rvol_ratio", 1.5), "rvol1.0-2.0")
        self.assertEqual(confidence.regime_bucket("rvol_ratio", 2.5), "rvol>2.0")
        self.assertEqual(confidence.regime_bucket("context_code", 3.0), "code=3")

    def test_hit_rate_at_a_horizon(self) -> None:
        indexed = [
            item(0, 100.0, 1.0),
            item(1, 101.0, 1.0),
            item(2, 100.0, -1.0),
        ]
        data, notes = confidence.hit_rate_at(indexed, "direction", 1)
        self.assertEqual(data["evaluated"], 2)
        self.assertEqual(data["hits"], 1)
        self.assertEqual(data["hit_rate"], 0.5)
        self.assertEqual(notes, [])


if __name__ == "__main__":
    unittest.main()

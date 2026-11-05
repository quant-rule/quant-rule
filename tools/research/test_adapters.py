#!/usr/bin/env python3
"""Unit tests for the research result adapters."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import adapters


def document(grade: str = "A", verdict: str = "pass") -> dict:
    return {
        "schema": "quantrule.model-prediction/v1",
        "spec_id": "M2.1",
        "family": "M2",
        "instrument": "BTC",
        "partition": "model_dev",
        "phase": "dev",
        "prediction_digest": "abc123",
        "horizon": 20,
        "cost_bps": 6.0,
        "grade": grade,
        "verdict": verdict,
        "metrics": {
            "n": 240,
            "clusters": 24,
            "hit_rate": 0.56,
            "base_rate": 0.50,
            "lift": 0.06,
            "lift_lcb": 0.02,
            "lcb_method": "cluster",
            "random_p": 0.01,
            "shuffle_p": 0.02,
            "back_lag_lift": 0.0,
            "replication": 2,
        },
        "limitations": ["Prediction evaluation only."],
    }


class ModelPredictionAdapterTests(unittest.TestCase):
    def test_a_passing_prediction_is_parsed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            (out / "model_prediction.json").write_text(
                json.dumps(document()), encoding="utf-8"
            )
            parsed = adapters.parse("model_prediction", out)
        self.assertEqual(parsed.verdict, "pass")
        self.assertEqual(parsed.metrics["n"], 240)
        self.assertEqual(parsed.metrics["grade"], "A")
        self.assertEqual(parsed.metrics["cost_bps"], 6.0)
        self.assertEqual(parsed.metrics["spec_id"], "M2.1")
        self.assertEqual(parsed.metrics["prediction_digest"], "abc123")

    def test_a_failing_prediction_is_parsed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            (out / "model_prediction.json").write_text(
                json.dumps(document("C", "fail")), encoding="utf-8"
            )
            parsed = adapters.parse("model_prediction", out)
        self.assertEqual(parsed.verdict, "fail")
        self.assertTrue(
            any("did not pass" in limitation for limitation in parsed.limitations)
        )


if __name__ == "__main__":
    unittest.main()

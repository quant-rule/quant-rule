#!/usr/bin/env python3
"""Unit tests for the v2 improvement manifest validator."""

from __future__ import annotations

import unittest

from common import validate_manifest


def manifest() -> dict:
    return {
        "schema": "quantrule.improvement-run/v2",
        "run_id": "MODEL-2026-11-07-01",
        "tier": "model",
        "hypothesis": "A model measurement has forward value.",
        "created_utc": "2026-11-07T12:00:00Z",
        "changed": "model",
        "frozen": ["strategy", "portfolio"],
        "partition": "model_dev",
        "run_config": "config/examples/run.json",
        "code_commit": "209530b",
        "seed": 0,
        "green_criteria": {"minimum_events": 200},
        "holdout_access": "none",
    }


class ValidateManifestTests(unittest.TestCase):
    def test_valid_model_manifest_passes(self) -> None:
        self.assertEqual(validate_manifest(manifest()), [])

    def test_v1_schema_is_rejected(self) -> None:
        value = manifest()
        value["schema"] = "quantrule.improvement-run/v1"
        errors = validate_manifest(value)
        self.assertTrue(any("quantrule.improvement-run/v2" in error for error in errors))

    def test_strategy_requires_strategy_config(self) -> None:
        value = manifest()
        value["run_id"] = "STRATEGY-2026-11-07-01"
        value["tier"] = "strategy"
        value["changed"] = "strategy"
        value["frozen"] = ["model", "portfolio"]
        value["partition"] = "strategy_dev"
        errors = validate_manifest(value)
        self.assertTrue(any("strategy_config: required" in error for error in errors))
        self.assertTrue(
            any("qualification_criteria: required" in error for error in errors)
        )

    def test_strategy_with_paths_passes(self) -> None:
        value = manifest()
        value["run_id"] = "STRATEGY-2026-11-07-01"
        value["tier"] = "strategy"
        value["changed"] = "strategy"
        value["frozen"] = ["model", "portfolio"]
        value["partition"] = "strategy_dev"
        value["strategy_config"] = "config/strategy/example.json"
        value["qualification_criteria"] = "config/qualification/example.json"
        self.assertEqual(validate_manifest(value), [])

    def test_bad_command_is_rejected(self) -> None:
        value = manifest()
        value["command"] = "backtest"
        errors = validate_manifest(value)
        self.assertTrue(any("command:" in error for error in errors))

    def test_bad_analytics_is_rejected(self) -> None:
        value = manifest()
        value["analytics"] = "yes"
        errors = validate_manifest(value)
        self.assertTrue(any("analytics:" in error for error in errors))


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for the current run-store adapter command construction."""

from __future__ import annotations

import unittest
from pathlib import Path

from run_current import commands_for, parse_stdout


def model_manifest() -> dict:
    return {
        "schema": "quantrule.improvement-run/v2",
        "run_id": "MODEL-2026-11-07-01",
        "tier": "model",
    }


def strategy_manifest() -> dict:
    return {
        "schema": "quantrule.improvement-run/v2",
        "run_id": "STRATEGY-2026-11-07-01",
        "tier": "strategy",
        "strategy_config": "config/strategy/example.json",
        "qualification_criteria": "config/qualification/example.json",
    }


class CommandConstructionTests(unittest.TestCase):
    def test_model_uses_report_model(self) -> None:
        commands = commands_for(
            model_manifest(),
            Path("/qr"),
            Path("/run"),
            {
                "run_config": "/config/run.json",
                "strategy_config": "",
                "qualification_criteria": "",
            },
        )
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][:3], ["/qr", "report", "model"])

    def test_strategy_uses_backtest_qualify_analytics(self) -> None:
        commands = commands_for(
            strategy_manifest(),
            Path("/qr"),
            Path("/run"),
            {
                "run_config": "/config/run.json",
                "strategy_config": "/config/strategy.json",
                "qualification_criteria": "/config/criteria.json",
            },
        )
        self.assertEqual(len(commands), 3)
        self.assertEqual(commands[0][:3], ["/qr", "backtest", "run"])
        self.assertEqual(commands[1][:2], ["/qr", "qualify"])
        self.assertEqual(commands[2][:3], ["/qr", "analytics", "evaluate"])

    def test_analytics_can_be_disabled(self) -> None:
        manifest = strategy_manifest()
        manifest["analytics"] = False
        commands = commands_for(
            manifest,
            Path("/qr"),
            Path("/run"),
            {
                "run_config": "/config/run.json",
                "strategy_config": "/config/strategy.json",
                "qualification_criteria": "/config/criteria.json",
            },
        )
        self.assertEqual(len(commands), 2)

    def test_parse_stdout_accepts_one_json_document(self) -> None:
        value, error = parse_stdout('{"ok": true}\n')
        self.assertEqual(value, {"ok": True})
        self.assertIsNone(error)

    def test_parse_stdout_rejects_text(self) -> None:
        value, error = parse_stdout("not json\n")
        self.assertIsNone(value)
        self.assertIsNotNone(error)


if __name__ == "__main__":
    unittest.main()

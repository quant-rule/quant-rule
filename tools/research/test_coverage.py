#!/usr/bin/env python3
"""Unit tests for the M2.x coverage checker."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import coverage

ROOT = Path(__file__).resolve().parents[2]
PROGRAM = ROOT / "config/research/PROGRAM-2026-11-05-03/program.json"
REGISTRY = ROOT / "docs/registry/indicators.json"


class CoverageTests(unittest.TestCase):
    def test_the_committed_program_is_covered(self) -> None:
        document = coverage.check(PROGRAM, REGISTRY, ROOT)
        self.assertTrue(document["ok"], document["errors"])
        self.assertEqual(document["counts"]["cells"], 15)
        self.assertEqual(document["counts"]["exploratory"], 10)
        self.assertEqual(document["counts"]["primary"], 5)
        self.assertEqual(document["counts"]["indicators"], 46)
        self.assertEqual(document["counts"]["events"], 143)
        self.assertEqual(document["counts"]["specs"], 60)
        self.assertEqual(document["missing_indicators"], [])
        self.assertEqual(document["missing_events"], [])

    def test_an_unused_registry_event_is_reported(self) -> None:
        registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
        registry["events"].append(
            {
                "id": "signal.audit_only",
                "indicator": "vwap",
                "role": "signal",
                "event_type": 3,
                "subtype": 1,
                "source": 19,
                "formula": "C > VWAP",
            }
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "registry.json"
            path.write_text(json.dumps(registry), encoding="utf-8")
            document = coverage.check(PROGRAM, path, ROOT)
        self.assertFalse(document["ok"])
        self.assertIn("signal.audit_only", document["missing_events"])

    def test_a_context_vote_is_reported(self) -> None:
        registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
        for event in registry["events"]:
            if event["id"] == "signal.rsi_threshold":
                event["role"] = "context"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "registry.json"
            path.write_text(json.dumps(registry), encoding="utf-8")
            document = coverage.check(PROGRAM, path, ROOT)
        self.assertFalse(document["ok"])
        self.assertTrue(
            any("is not a signal" in error for error in document["errors"])
        )

    def test_condition_events_read_nested_expressions(self) -> None:
        value = {
            "all": [
                {"event": {"id": "signal.session_open"}},
                {"not": {"event": {"id": "signal.session_close"}}},
            ]
        }
        self.assertEqual(
            coverage.condition_events(value),
            ["signal.session_open", "signal.session_close"],
        )


if __name__ == "__main__":
    unittest.main()

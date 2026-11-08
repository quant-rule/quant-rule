#!/usr/bin/env python3
"""Tests for the MR7 opening-range sweep grid, coverage, and builder."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import build_mr5_program
import coverage_mr5
import program as program_module

ROOT = Path(__file__).resolve().parents[2]
PROGRAM_MR7 = ROOT / "config/research/PROGRAM-2026-11-08-03/program.json"
PROGRAM_M2 = ROOT / "config/research/PROGRAM-2026-11-05-03/program.json"
REGISTRY = ROOT / "docs/registry/indicators.json"


class Mr7Tests(unittest.TestCase):
    def test_real_mr7_programme_is_covered(self) -> None:
        document = coverage_mr5.check_mr5(
            PROGRAM_MR7, REGISTRY, PROGRAM_M2, ROOT, family="MR7"
        )
        self.assertTrue(document["ok"], document["errors"])
        self.assertEqual(document["family"], "MR7")
        self.assertEqual(document["counts"]["cells"], 15)
        self.assertEqual(document["counts"]["primary"], 5)
        self.assertEqual(document["counts"]["indicators"], 46)
        self.assertEqual(document["counts"]["events"], 145)
        self.assertEqual(document["counts"]["specs"], 99)
        self.assertEqual(document["inherited"], "PROGRAM-2026-11-05-03")
        timeframes = {cell["timeframe"] for cell in document["cells"]}
        self.assertEqual(timeframes, {900, 3600})
        instruments = {item for cell in document["cells"] for item in cell["instruments"]}
        self.assertEqual(
            instruments, {"XAU", "BTC", "SP500", "EURUSD", "ETH", "COPPER", "TSLA"}
        )

    def test_builder_writes_session_model_id_and_horizon(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = root / "base-model.json"
            base.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.model/v4",
                        "defaults": {"rsi": {"period": 14, "active": True}},
                    }
                ),
                encoding="utf-8",
            )
            grid = {
                "schema": "quantrule.research-grid/v1",
                "program_id": "PROGRAM-2026-11-08-96",
                "family": "MR7",
                "run_prefix": "mr7",
                "model_id": "M1.1",
                "premise": "Builder test",
                "economic_rationale": "Fixture",
                "base_model": "base-model.json",
                "timeframe": 900,
                "instruments": ["XAU"],
                "hypotheses": [
                    {
                        "id": "MR7.1",
                        "primary": True,
                        "hypothesis": "Fixture sweep",
                        "horizon": 10,
                        "events": ["signal.opening_range_sweep"],
                        "session": {
                            "zone": "london",
                            "start_hour": 8,
                            "end_hour": 16,
                            "opening_range_bars": 2,
                            "sweep_atr": 0.0,
                        },
                        "condition": {
                            "schema": "quantrule.model-condition/v1",
                            "kind": "column",
                            "column": "opening_range_atr",
                            "min": 0.0,
                        },
                        "variants": [
                            {"suffix": "C01", "model": {}, "condition": {}, "session": {}},
                            {
                                "suffix": "C02",
                                "model": {},
                                "condition": {},
                                "session": {"sweep_atr": 0.2},
                            },
                        ],
                    }
                ],
                "correction": {"method": "bonferroni", "primary_cells": 1},
            }
            grid_path = root / "grid.json"
            grid_path.write_text(json.dumps(grid), encoding="utf-8")
            out = root / "PROGRAM-2026-11-08-96"
            loaded = build_mr5_program.load_grid(grid_path)
            build_mr5_program.build_program(loaded, out, root)
            program = program_module.load_program(out / "program.json", repo_root=root)
            self.assertEqual(len(program.cells), 2)
            command = program.cells[0].command
            self.assertEqual(command[command.index("--horizon") + 1], "10")
            spec = json.loads((out / "cells/MR7.1-C02/spec-XAU-900.json").read_text())
            self.assertEqual(spec["session"]["sweep_atr"], 0.2)
            self.assertEqual(spec["session"]["opening_range_bars"], 2)
            self.assertEqual(spec["prediction"]["events"], ["signal.opening_range_sweep"])
            self.assertFalse((out / "runs").exists())


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Tests for the MR5 reference grid, coverage, and builder."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import build_mr5_program
import coverage_mr5
import program as program_module

ROOT = Path(__file__).resolve().parents[2]
PROGRAM_MR5 = ROOT / "config/research/reference-mr5/program.json"
PROGRAM_M2 = ROOT / "config/research/PROGRAM-2026-11-05-03/program.json"
REGISTRY = ROOT / "docs/registry/indicators.json"


class Mr5Tests(unittest.TestCase):
    def test_reference_grid_is_covered(self) -> None:
        document = coverage_mr5.check_mr5(PROGRAM_MR5, REGISTRY, PROGRAM_M2, ROOT)
        self.assertTrue(document["ok"], document["errors"])
        self.assertEqual(document["family"], "MR5")
        self.assertEqual(document["counts"]["cells"], 2)
        self.assertEqual(document["counts"]["primary"], 1)
        self.assertEqual(document["counts"]["indicators"], 46)
        self.assertEqual(document["counts"]["events"], 143)
        self.assertEqual(document["counts"]["specs"], 2)
        self.assertEqual(document["inherited"], "PROGRAM-2026-11-05-03")

    def test_builder_writes_a_grid(self) -> None:
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
                "program_id": "PROGRAM-2026-11-08-98",
                "premise": "Builder test",
                "economic_rationale": "Fixture",
                "base_model": "base-model.json",
                "timeframe": 900,
                "instruments": ["XAU"],
                "hypotheses": [
                    {
                        "id": "MR5.1",
                        "primary": True,
                        "hypothesis": "Fixture hypothesis",
                        "events": ["signal.rsi_threshold"],
                        "condition": {
                            "schema": "quantrule.model-condition/v1",
                            "kind": "column",
                            "column": "level_touches",
                            "min": 1,
                        },
                        "variants": [
                            {"suffix": "C01", "model": {}, "condition": {}},
                            {
                                "suffix": "C02",
                                "model": {"defaults": {"rsi": {"period": 10}}},
                                "condition": {"level_age_max": 12},
                            },
                        ],
                    }
                ],
                "correction": {"method": "bonferroni", "primary_cells": 1},
            }
            grid_path = root / "grid.json"
            grid_path.write_text(json.dumps(grid), encoding="utf-8")
            out = root / "PROGRAM-2026-11-08-98"
            loaded = build_mr5_program.load_grid(grid_path)
            build_mr5_program.build_program(loaded, out, root)
            program = program_module.load_program(out / "program.json", repo_root=root)
            self.assertEqual(len(program.cells), 2)
            self.assertEqual(len(program.primary_cells), 1)
            self.assertEqual(program.correction["primary_cells"], 1)
            self.assertTrue((out / "cells/MR5.1-C01/model.json").is_file())
            self.assertTrue((out / "cells/MR5.1-C02/condition.json").is_file())
            model = json.loads((out / "cells/MR5.1-C02/model.json").read_text())
            self.assertEqual(model["defaults"]["rsi"]["period"], 10)
            condition = json.loads((out / "cells/MR5.1-C02/condition.json").read_text())
            self.assertEqual(condition["level_age_max"], 12)


if __name__ == "__main__":
    unittest.main()

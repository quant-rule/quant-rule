#!/usr/bin/env python3
"""Tests for the MR6 ordered-confirmation grid, coverage, and builder."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import build_mr5_program
import coverage_mr5
import program as program_module

ROOT = Path(__file__).resolve().parents[2]
PROGRAM_MR6 = ROOT / "config/research/PROGRAM-2026-11-08-02/program.json"
PROGRAM_M2 = ROOT / "config/research/PROGRAM-2026-11-05-03/program.json"
REGISTRY = ROOT / "docs/registry/indicators.json"


class Mr6Tests(unittest.TestCase):
    def test_real_mr6_programme_is_covered(self) -> None:
        document = coverage_mr5.check_mr5(
            PROGRAM_MR6, REGISTRY, PROGRAM_M2, ROOT, family="MR6"
        )
        self.assertTrue(document["ok"], document["errors"])
        self.assertEqual(document["family"], "MR6")
        self.assertEqual(document["counts"]["cells"], 15)
        self.assertEqual(document["counts"]["primary"], 5)
        self.assertEqual(document["counts"]["indicators"], 46)
        self.assertEqual(document["counts"]["events"], 143)
        self.assertEqual(document["counts"]["specs"], 75)
        self.assertEqual(document["inherited"], "PROGRAM-2026-11-05-03")
        timeframes = {cell["timeframe"] for cell in document["cells"]}
        self.assertEqual(timeframes, {300, 900})
        instruments = {item for cell in document["cells"] for item in cell["instruments"]}
        self.assertEqual(instruments, {"XAU", "BTC", "SP500", "EURUSD", "ETH"})

    def test_builder_synthesizes_an_extra_instrument(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runs = root / "runs"
            source = runs / "XAU" / "2018"
            source.mkdir(parents=True)
            (runs / "XAU" / "policy.json").write_text(
                json.dumps(
                    {
                        "schema": "quantrule.policy/v1",
                        "id": "POL1.0",
                        "universe": {"tickets": ["XAUUSD"]},
                    }
                ),
                encoding="utf-8",
            )
            (source / "run.json").write_text(
                json.dumps(
                    {
                        "schema": "quantrule.run/v2",
                        "name": "mr6-xau-2018",
                        "output_root": "../../data/research/PROGRAM-2026-11-08-01/tmp",
                    }
                ),
                encoding="utf-8",
            )
            (source / "data.json").write_text(
                json.dumps(
                    {
                        "schema": "quantrule.data/v3",
                        "instrument": "XAUUSD",
                        "data_version": "mr6-xau-m1-2018-01-01_2018-12-31",
                        "source": {"kind": "csv", "path": "../../../../data/sources/XAU.csv"},
                    }
                ),
                encoding="utf-8",
            )
            build_mr5_program.synthesize_instrument_runs(
                runs,
                {
                    "code": "ETH",
                    "template": "XAU",
                    "instrument": "ETHUSD",
                    "source": "ETHUSD_M1_2018-01-01_2024-12-31.csv",
                },
                "mr6",
                "PROGRAM-2026-11-08-02",
            )
            run = json.loads((runs / "ETH" / "2018" / "run.json").read_text())
            data = json.loads((runs / "ETH" / "2018" / "data.json").read_text())
            policy = json.loads((runs / "ETH" / "policy.json").read_text())
            self.assertEqual(run["name"], "mr6-eth-2018")
            self.assertEqual(data["instrument"], "ETHUSD")
            self.assertEqual(data["data_version"], "mr6-eth-m1-2018-01-01_2018-12-31")
            self.assertEqual(
                data["source"]["path"],
                "../../../../../../data/sources/ETHUSD_M1_2018-01-01_2024-12-31.csv",
            )
            self.assertEqual(policy["universe"]["tickets"], ["ETHUSD"])

    def test_builder_writes_family_timeframe_and_event_lists(self) -> None:
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
                "program_id": "PROGRAM-2026-11-08-97",
                "family": "MR6",
                "run_prefix": "mr6",
                "premise": "Builder test",
                "economic_rationale": "Fixture",
                "base_model": "base-model.json",
                "timeframe": 900,
                "instruments": ["XAU", "ETH"],
                "hypotheses": [
                    {
                        "id": "MR6.1",
                        "primary": True,
                        "hypothesis": "Fixture ordered confirmation",
                        "events": ["signal.rvol_expansion"],
                        "condition": {
                            "schema": "quantrule.model-condition/v1",
                            "kind": "sequence",
                            "first_events": ["signal.di_cross", "signal.trend_continuation"],
                            "second_events": ["signal.rvol_expansion", "signal.obv_cross"],
                            "within_bars": 5,
                            "min_gap_bars": 1,
                            "direction": "same",
                        },
                        "variants": [
                            {"suffix": "C01", "model": {}, "condition": {}},
                            {"suffix": "C02", "model": {}, "condition": {"within_bars": 3}},
                        ],
                    },
                    {
                        "id": "MR6.2",
                        "primary": False,
                        "hypothesis": "Fixture 5m cell",
                        "timeframe": 300,
                        "events": ["signal.di_cross"],
                        "condition": {
                            "schema": "quantrule.model-condition/v1",
                            "kind": "sequence",
                            "first_events": ["signal.rvol_expansion"],
                            "second_events": ["signal.di_cross"],
                            "within_bars": 5,
                            "min_gap_bars": 1,
                            "direction": "same",
                        },
                        "variants": [
                            {"suffix": "C01", "model": {}, "condition": {}},
                        ],
                    },
                ],
                "correction": {"method": "bonferroni", "primary_cells": 1},
            }
            grid_path = root / "grid.json"
            grid_path.write_text(json.dumps(grid), encoding="utf-8")
            out = root / "PROGRAM-2026-11-08-97"
            loaded = build_mr5_program.load_grid(grid_path)
            build_mr5_program.build_program(loaded, out, root)
            program = program_module.load_program(out / "program.json", repo_root=root)
            self.assertEqual(len(program.cells), 3)
            self.assertEqual(len(program.primary_cells), 1)
            self.assertTrue((out / "cells/MR6.1-C01/spec-XAU-900.json").is_file())
            self.assertTrue((out / "cells/MR6.1-C01/spec-ETH-900.json").is_file())
            self.assertTrue((out / "cells/MR6.2-C01/spec-XAU-300.json").is_file())
            coverage = json.loads((out / "cells/MR6.1-C01/coverage.json").read_text())
            self.assertEqual(coverage["timeframe"], 900)
            self.assertEqual(
                coverage["condition_events"],
                [
                    "signal.di_cross",
                    "signal.trend_continuation",
                    "signal.rvol_expansion",
                    "signal.obv_cross",
                ],
            )
            spec = json.loads((out / "cells/MR6.2-C01/spec-XAU-300.json").read_text())
            self.assertEqual(spec["family"], "MR6")
            self.assertEqual(spec["timeframes"], [300])
            command = program.cell("MR6.2-C01").command
            self.assertEqual(command[command.index("--family") + 1], "MR6")
            self.assertEqual(command[command.index("--timeframe") + 1], "300")


if __name__ == "__main__":
    unittest.main()

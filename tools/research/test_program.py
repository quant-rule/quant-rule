#!/usr/bin/env python3
"""Unit tests for the research program manifest."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import program


def cell(cell_id: str, primary: bool = True, **extra: object) -> dict:
    value = {
        "cell_id": cell_id,
        "hypothesis": f"Hypothesis {cell_id}",
        "primary": primary,
        "config_dir": f"cells/{cell_id}",
        "command": ["python3", "-c", "pass", "{out}"],
        "result_parser": "none",
    }
    value.update(extra)
    return value


def document(cells: list[dict] | None = None, **extra: object) -> dict:
    values = cells if cells is not None else [cell("CELL-01"), cell("CELL-02")]
    primary = sum(1 for item in values if item.get("primary"))
    value = {
        "schema": program.SCHEMA,
        "program_id": "PROGRAM-2026-11-04-90",
        "premise": "A test premise.",
        "economic_rationale": "A test rationale.",
        "tier": "model",
        "changed": "model",
        "frozen": ["strategy", "portfolio"],
        "partition_dev": "model_dev",
        "partition_validation": "model_validation",
        "costs": {"commission_bps": 4, "slippage_bps": 2, "funding": 0},
        "green_criteria": {"min_events": 200},
        "correction": {"method": "bonferroni", "primary_cells": primary},
        "max_parallel": 2,
        "iteration_cap": 1,
        "validation_access": "none",
        "stop_rule": "Stop when no primary cell passes.",
        "cells": values,
    }
    value.update(extra)
    return value


def write(root: Path, value: dict) -> Path:
    for item in value["cells"]:
        (root / item["config_dir"]).mkdir(parents=True, exist_ok=True)
    path = root / "program.json"
    path.write_text(json.dumps(value), encoding="utf-8")
    return path


class ProgramTests(unittest.TestCase):
    def test_valid_manifest_loads(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, document())
            loaded = program.load_program(path, repo_root=root)
            self.assertEqual(loaded.program_id, "PROGRAM-2026-11-04-90")
            self.assertEqual(len(loaded.cells), 2)
            self.assertEqual(len(loaded.primary_cells), 2)

    def test_duplicate_cell_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, document([cell("CELL-01"), cell("CELL-01")]))
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)

    def test_primary_correction_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = document()
            value["correction"]["primary_cells"] = 1
            path = write(root, value)
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)

    def test_missing_config_dir_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = document()
            value["cells"][0]["config_dir"] = "cells/missing"
            path = write(root, value)
            (root / "cells" / "missing").rmdir()
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)

    def test_command_requires_out(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = document()
            value["cells"][0]["command"] = ["python3", "-c", "pass"]
            path = write(root, value)
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)

    def test_unknown_parser_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = document()
            value["cells"][0]["result_parser"] = "unknown"
            path = write(root, value)
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)

    def test_validation_requires_one_shot_access(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, document())
            loaded = program.load_program(path, repo_root=root)
            with self.assertRaises(program.ProgramError):
                program.require_phase(loaded, "validation")
            value = document(validation_access="one_shot")
            path = write(root, value)
            loaded = program.load_program(path, repo_root=root)
            program.require_phase(loaded, "validation")

    def test_changed_must_match_tier(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = document(changed="strategy")
            path = write(root, value)
            with self.assertRaises(program.ProgramError):
                program.load_program(path, repo_root=root)


if __name__ == "__main__":
    unittest.main()

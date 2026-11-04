#!/usr/bin/env python3
"""Unit tests for the research program registry."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import program
import registry
from test_program import document, write


def report_document(decision: str, candidate: str | None) -> dict:
    return {
        "schema": "quantrule.research-program-results/v1",
        "program_id": "PROGRAM-2026-11-04-90",
        "decision": decision,
        "candidate": candidate,
        "correction": {"method": "bonferroni", "primary_cells": 2},
        "cells": [
            {
                "cell_id": "CELL-01",
                "primary": True,
                "verdict": "fail",
                "metrics": {"n": 100, "clusters": 20, "lift": 0.01, "lift_lcb": -0.01, "grade": "C"},
            }
        ],
    }


class RegistryTests(unittest.TestCase):
    def test_no_edge_status(self) -> None:
        self.assertEqual(
            registry.status(report_document("no_edge", None), None, None), "no_edge"
        )

    def test_candidate_status(self) -> None:
        self.assertEqual(
            registry.status(report_document("candidate", "CELL-01"), None, None),
            "candidate",
        )

    def test_accepted_requires_both_fields(self) -> None:
        with self.assertRaises(program.ProgramError):
            registry.status(report_document("candidate", "CELL-01"), "human", None)
        self.assertEqual(
            registry.status(
                report_document("candidate", "CELL-01"), "human", "2026-11-04"
            ),
            "accepted",
        )

    def test_markdown_records_the_program(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write(root, document())
            loaded = program.load_program(path, repo_root=root)
            text = registry.markdown(
                loaded,
                report_document("no_edge", None),
                "no_edge",
                None,
                None,
            )
            self.assertIn("# Program PROGRAM-2026-11-04-90", text)
            self.assertIn("Status: **no_edge**", text)
            self.assertIn("CELL-01", text)


if __name__ == "__main__":
    unittest.main()

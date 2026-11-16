#!/usr/bin/env python3
"""Unit tests for the discovery-gate command checks.

The workflows and ``AGENTS.md`` are current procedure, so a command they invoke
must be in the registry. The tests seed a retired command in a temporary
workflow and a stale normative list so the guard cannot silently stop matching.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_command_surface


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class WorkflowInvocationTests(unittest.TestCase):
    def test_retired_command_in_a_workflow_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "strategy.md"
            write(path, "```bash\nquantrule backtest run --config run.json\n```\n")
            problems = check_command_surface.document_unimplemented(
                {"run submit"}, [path]
            )
            self.assertEqual(len(problems), 1, problems)
            self.assertIn("backtest run", problems[0])

    def test_implemented_command_in_a_workflow_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "strategy.md"
            write(path, "```bash\nquantrule run submit dv_1 cfg_1\n```\n")
            self.assertEqual(
                check_command_surface.document_unimplemented({"run submit"}, [path]),
                [],
            )


class InvocationGrammarTests(unittest.TestCase):
    def test_wrong_flag_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "runbook.md"
            write(path, "```bash\nquantrule run submit --data-version dv_1 --config cfg_1\n```\n")
            problems = check_command_surface.invocation_errors(
                check_command_surface.help_commands(), [path]
            )
            self.assertTrue(any("unknown option '--data-version'" in p for p in problems), problems)
            self.assertTrue(any("unknown option '--config'" in p for p in problems), problems)

    def test_positional_command_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "runbook.md"
            write(path, "```bash\nquantrule run submit dv_1 cfg_1\n```\n")
            self.assertEqual(
                check_command_surface.invocation_errors(
                    check_command_surface.help_commands(), [path]
                ),
                [],
            )

    def test_retired_command_in_a_current_config_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "contract.md"
            write(
                path,
                "- Status: Current\n\nThe `strategy run` command writes a bundle.\n",
            )
            problems = check_command_surface.retired_backticks([path])
            self.assertEqual(len(problems), 1, problems)
            self.assertIn("strategy run", problems[0])

    def test_retired_command_in_a_historical_config_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "contract.md"
            write(
                path,
                "- Status: Historical\n\nThe `strategy run` command was retired.\n",
            )
            self.assertEqual(check_command_surface.retired_backticks([path]), [])


class NormativeListTests(unittest.TestCase):
    def test_the_surface_paragraph_yields_its_commands(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "AGENTS.md"
            write(
                path,
                "Intro.\n\nThe current tree implements `health` and `run submit`.\n\n"
                "A later paragraph.\n",
            )
            self.assertEqual(
                check_command_surface.normative_commands(path),
                {"health", "run submit"},
            )

    def test_a_missing_surface_paragraph_yields_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "AGENTS.md"
            write(path, "No surface here.\n")
            self.assertEqual(check_command_surface.normative_commands(path), set())


if __name__ == "__main__":
    unittest.main()

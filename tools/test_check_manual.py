#!/usr/bin/env python3
"""Unit tests for the user-manual discovery checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_manual

ROOT = Path(__file__).resolve().parent.parent

README = """# QuantRule User Manual

- Status: Current

## 2. Chapters

| # | Chapter | File | Answers | Status |
|---:|---|---|---|---|
| 1 | First run | [01-first-run.md](01-first-run.md) | How to start | Current |
| 9 | Command reference | [09-command-reference.md](09-command-reference.md) | Every command | Current |
"""

CHAPTER = """# First run

- Status: Current

## Goal

Start the product.

## Before you start

A stack.

## Steps

| Shell | Browser |
|---|---|
| `quantrule health` | Connection |

## What you see

The health table.

## What it proves

**Diagnostic.** It is a read-only check.

## Failure

| Code | Action |
|---|---|
| `profile.not_found` | Create the profile |
"""

COMMANDS = """# Command reference

- Status: Current

| Command | Usage |
|---|---|
| `health` | `health` |
| `run submit` | `run submit <data_version> <config_id>` |
"""

CLIENT = """# Web Client

## 1. Screens

| Screen | Component | Shows | Empty state |
|---|---|---|---|
| Connection | `ConnectionPanel.svelte` | Gateway state | `unreachable` |
| Workbench | `WorkbenchPanel.svelte` | Tabs | "No stage runs." |
"""

GUI = """# GUI reference

| Screen / tab | Component | Shows |
|---|---|---|
| Connection | `ConnectionPanel.svelte` | Gateway state |
| Workbench | `WorkbenchPanel.svelte` | Tabs |
| Workbench — Data | `DatasetsPanel.svelte` | Datasets |
| Workbench — Model | `ModelStagePanel.svelte` | Stages |
"""

HELP = """struct CommandHelp { name: &'static str }

const COMMANDS: &[CommandHelp] = &[
    CommandHelp {
        name: "health",
        usage: "health",
        summary: "Read health.",
        output: "health",
        notes: &[],
    },
    CommandHelp {
        name: "run submit",
        usage: "run submit <data_version> <config_id>",
        summary: "Submit.",
        output: "run id",
        notes: &[],
    },
];
"""

WORKBENCH = """<button role="tab" aria-selected={tab === "data"} onclick={() => switchTab("data")}>
  Data
</button>
<button role="tab" aria-selected={tab === "model"} onclick={() => switchTab("model")}>
  Model
</button>
"""


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def seed(root: Path) -> None:
    manual = root / "docs" / "manual"
    write(manual / "README.md", README)
    write(manual / "01-first-run.md", CHAPTER)
    write(manual / "09-command-reference.md", COMMANDS)
    write(manual / "10-gui-reference.md", GUI)
    write(root / "docs" / "architecture" / "client.md", CLIENT)
    write(
        root / "crates" / "quantrule-shell" / "src" / "command" / "help.rs",
        HELP,
    )
    write(
        root / "apps" / "web" / "src" / "lib" / "WorkbenchPanel.svelte",
        WORKBENCH,
    )


class ManualTests(unittest.TestCase):
    def test_the_current_tree_passes(self) -> None:
        self.assertEqual(check_manual.check(ROOT), [])

    def test_a_minimal_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            self.assertEqual(check_manual.check(root), [])

    def test_a_missing_chapter_file_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            (root / "docs" / "manual" / "01-first-run.md").unlink()
            problems = check_manual.check(root)
            self.assertTrue(any("chapter file is missing" in p for p in problems), problems)

    def test_a_missing_command_row_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(root / "docs" / "manual" / "09-command-reference.md", "# Reference\n")
            problems = check_manual.check(root)
            self.assertTrue(any("no row for 'health'" in p for p in problems), problems)

    def test_a_usage_mismatch_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "manual" / "09-command-reference.md",
                COMMANDS.replace("`run submit <data_version> <config_id>`", "`run submit <id>`"),
            )
            problems = check_manual.check(root)
            self.assertTrue(any("usage for 'run submit'" in p for p in problems), problems)

    def test_a_missing_screen_row_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(root / "docs" / "manual" / "10-gui-reference.md", "# GUI\n")
            problems = check_manual.check(root)
            self.assertTrue(any("no row for screen 'Connection'" in p for p in problems), problems)

    def test_a_missing_section_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "manual" / "01-first-run.md",
                CHAPTER.replace("## Failure", "## Problems"),
            )
            problems = check_manual.check(root)
            self.assertTrue(any("missing the 'Failure' section" in p for p in problems), problems)

    def test_an_invalid_evidence_label_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "manual" / "01-first-run.md",
                CHAPTER.replace("**Diagnostic.**", "**Outcome.**"),
            )
            problems = check_manual.check(root)
            self.assertTrue(any("names no evidence label" in p for p in problems), problems)

    def test_a_missing_workbench_tab_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "apps" / "web" / "src" / "lib" / "WorkbenchPanel.svelte",
                WORKBENCH.replace("Model", "Portfolio"),
            )
            problems = check_manual.check(root)
            self.assertTrue(
                any("no row for 'Workbench — Portfolio'" in p for p in problems), problems
            )


if __name__ == "__main__":
    unittest.main()

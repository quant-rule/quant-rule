#!/usr/bin/env python3
"""Unit tests for the client-document and status-count checker."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_client_docs

ROOT = Path(__file__).resolve().parent.parent

CLIENT = """# Web Client

## 1. Screens

| Screen | Component | Shows | Empty state |
|---|---|---|---|
| Connection | `ConnectionPanel.svelte` | Gateway state | `unreachable` |
| Runs | `RunsPanel.svelte` | Run rows | "No runs." |

## 2. Stores

| Store | Holds | Polling |
|---|---|---|
| `runs.svelte.ts` | run rows | 30 s |
"""

CAPABILITIES = """export const SECTIONS = [
  { id: "connection", label: "Connection", hint: "Gateway" },
  { id: "runs", label: "Runs", hint: "Runs" },
] as const;

export const COMMANDS = [
  { command: "health", section: "connection", control: "Health", component: "ConnectionPanel.svelte", testId: "health-report" },
  { command: "run list", section: "runs", control: "Runs table", component: "RunsPanel.svelte", testId: "run-table" },
];
"""

README = (
    "| Indicator verification matrix | Current; 2 rows, generated from the evidence map | path |\n"
    "| Event verification matrix | Current; 3 rows, generated from the evidence map | path |\n"
)

REGISTRY = {
    "indicators": [{"id": "ema"}, {"id": "rsi"}],
    "events": [{"id": "level.test"}, {"id": "level.cross"}, {"id": "signal.rsi_threshold"}],
}


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def seed(root: Path) -> None:
    write(root / "docs" / "architecture" / "client.md", CLIENT)
    write(root / "docs" / "model" / "README.md", README)
    write(
        root / "docs" / "registry" / "indicators.json",
        json.dumps(REGISTRY),
    )
    library = root / "apps" / "web" / "src" / "lib"
    write(library / "capabilities.ts", CAPABILITIES)
    write(
        library / "ConnectionPanel.svelte",
        '<section data-testid="health-report"></section>\n',
    )
    write(
        library / "RunsPanel.svelte",
        '<section data-testid="run-table"></section>\n',
    )
    write(library / "runs.svelte.ts", "export const runs = {};\n")


class ClientDocumentTests(unittest.TestCase):
    def test_the_current_tree_passes(self) -> None:
        self.assertEqual(check_client_docs.check(ROOT), [])

    def test_a_minimal_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            self.assertEqual(check_client_docs.check(root), [])

    def test_a_missing_component_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            (root / "apps" / "web" / "src" / "lib" / "ConnectionPanel.svelte").unlink()
            problems = check_client_docs.check(root)
            self.assertTrue(
                any("missing component 'ConnectionPanel.svelte'" in p for p in problems)
            )

    def test_a_stale_indicator_count_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "model" / "README.md",
                README.replace("Current; 2 rows", "Current; 46 rows"),
            )
            problems = check_client_docs.check(root)
            self.assertTrue(
                any("46 indicator rows, registry has 2" in p for p in problems)
            )

    def test_a_stale_event_count_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "model" / "README.md",
                README.replace("Current; 3 rows", "Current; 143 rows"),
            )
            problems = check_client_docs.check(root)
            self.assertTrue(
                any("143 event rows, registry has 3" in p for p in problems)
            )

    def test_a_navigation_label_without_a_screen_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "apps" / "web" / "src" / "lib" / "capabilities.ts",
                CAPABILITIES.replace(
                    "] as const;",
                    '  { id: "help", label: "Help", hint: "Reference" },\n] as const;',
                ),
            )
            problems = check_client_docs.check(root)
            self.assertTrue(
                any("navigation label 'Help' has no screen row" in p for p in problems)
            )

    def test_a_capability_without_a_test_id_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "apps" / "web" / "src" / "lib" / "ConnectionPanel.svelte",
                "<section></section>\n",
            )
            problems = check_client_docs.check(root)
            self.assertTrue(
                any("health-report" in problem and "data-testid" in problem for problem in problems)
            )

    def test_a_missing_store_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            (root / "apps" / "web" / "src" / "lib" / "runs.svelte.ts").unlink()
            problems = check_client_docs.check(root)
            self.assertTrue(any("store 'runs.svelte.ts' does not exist" in p for p in problems))

    def test_a_missing_screen_table_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed(root)
            write(
                root / "docs" / "architecture" / "client.md",
                CLIENT.replace(
                    "| Screen | Component | Shows | Empty state |\n|---|---|---|---|\n",
                    "",
                ),
            )
            problems = check_client_docs.check(root)
            self.assertTrue(any("no '| Screen | Component" in p for p in problems))


if __name__ == "__main__":
    unittest.main()

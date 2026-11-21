#!/usr/bin/env python3
"""Unit tests for the shared checker gate.

`tools/gate.sh` and the CI workflow must run the same checker list and the same
checker-test list. The tests also seed a doc-length failure and a failing checker
test to prove the shared runner propagates both.
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent


def lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


class SharedListTests(unittest.TestCase):
    def test_every_checker_is_listed(self) -> None:
        listed = set(lines(TOOLS / "checkers.txt"))
        present = {path.stem for path in TOOLS.glob("check_*.py")}
        self.assertEqual(listed, present)

    def test_every_checker_test_is_listed(self) -> None:
        listed = set(lines(TOOLS / "checker_tests.txt"))
        present = {path.stem for path in TOOLS.glob("test_check_*.py")}
        self.assertEqual(listed, present)

    def test_both_entry_points_call_the_shared_runner(self) -> None:
        self.assertIn("tools/checkers.sh", (ROOT / "tools/gate.sh").read_text())
        self.assertIn(
            "tools/checkers.sh",
            (ROOT / ".github/workflows/ci.yml").read_text(),
        )


class SeededFailureTests(unittest.TestCase):
    def test_a_seeded_doc_length_failure_fails_the_runner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "tools").mkdir()
            shutil.copy(TOOLS / "checkers.sh", root / "tools/checkers.sh")
            shutil.copy(TOOLS / "check_doc_length.py", root / "tools/check_doc_length.py")
            (root / "tools/checkers.txt").write_text("check_doc_length\n", encoding="utf-8")
            (root / "tools/checker_tests.txt").write_text("", encoding="utf-8")
            architecture = root / "docs/architecture"
            architecture.mkdir(parents=True)
            (architecture / "long.md").write_text(
                "# Long\n\n" + "line\n" * 200, encoding="utf-8"
            )
            result = subprocess.run(
                ["bash", "tools/checkers.sh", "check_doc_length"],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("over", result.stdout + result.stderr)

    def test_a_seeded_checker_test_failure_fails_the_runner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "tools").mkdir()
            shutil.copy(TOOLS / "checkers.sh", root / "tools/checkers.sh")
            (root / "tools/checkers.txt").write_text("", encoding="utf-8")
            (root / "tools/checker_tests.txt").write_text(
                "test_seed_failure\n", encoding="utf-8"
            )
            (root / "tools/test_seed_failure.py").write_text(
                "import unittest\n\n"
                "class SeedFailure(unittest.TestCase):\n"
                "    def test_fails(self):\n"
                "        self.fail('seeded checker-test failure')\n",
                encoding="utf-8",
            )
            result = subprocess.run(
                ["bash", "tools/checkers.sh", "--tests-only"],
                cwd=root,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("seeded checker-test failure", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()

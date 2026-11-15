#!/usr/bin/env python3
"""Unit tests for the Python product gate.

Every test builds a minimal tree so a seeded product file or interpreter
reference cannot silently stop failing the gate. The governance allowlist and
the empty tree are tested the same way, so the gate cannot widen by accident.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_python


def write(path: Path, text: str = "") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class ProductPythonFileTests(unittest.TestCase):
    def test_a_python_file_under_each_product_root_fails(self) -> None:
        for top in ("crates", "apps", "docker"):
            with self.subTest(top=top), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                write(root / top / "example" / "tool.py")
                errors = check_python.check(root)
                self.assertEqual(len(errors), 1, errors)
                self.assertTrue(
                    errors[0].startswith(f"{top}/example/tool.py:"), errors[0]
                )

    def test_a_python_file_beside_the_dockerfile_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "Dockerfile", "FROM debian:trixie-slim\n")
            write(root / "config" / "generated.py")
            errors = check_python.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("config/generated.py", errors[0])

    def test_a_python_file_in_the_governance_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "tools" / "check_example.py")
            write(root / "tools" / "improvement" / "cleanup.py")
            self.assertEqual(check_python.check(root), [])

    def test_a_python_file_in_the_history_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "audits" / "2026-11-15" / "record.py")
            self.assertEqual(check_python.check(root), [])

    def test_build_output_is_not_scanned(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "apps" / "web" / "node_modules" / "pkg" / "index.py")
            write(root / "crates" / "example" / "target" / "build.py")
            self.assertEqual(check_python.check(root), [])

    def test_an_empty_product_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            self.assertEqual(check_python.check(Path(temporary)), [])


class InterpreterReferenceTests(unittest.TestCase):
    def test_a_dockerfile_install_fails_with_the_line(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "Dockerfile",
                "FROM debian:trixie-slim\nRUN apt-get install -y python3\n",
            )
            errors = check_python.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertTrue(errors[0].startswith("Dockerfile:2:"), errors[0])
            self.assertIn("python3", errors[0])

    def test_a_product_script_invocation_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docker" / "serve.sh", '#!/bin/sh\npython3 -m http.server\n')
            errors = check_python.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertTrue(errors[0].startswith("docker/serve.sh:2:"), errors[0])

    def test_a_versioned_interpreter_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "crates" / "example" / "build.rs", "python3.13 -V\n")
            errors = check_python.check(root)
            self.assertEqual(len(errors), 1, errors)
            self.assertIn("python3.13", errors[0])

    def test_a_rust_identifier_and_prose_do_not_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "example" / "src" / "lib.rs",
                "//! Ported from an independent Python implementation.\n"
                "fn python_number() -> u8 { 1 }\n",
            )
            self.assertEqual(check_python.check(root), [])

    def test_the_image_check_script_is_allowlisted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docker" / "check-no-python.sh",
                "for name in python python3; do command -v \"$name\"; done\n",
            )
            self.assertEqual(check_python.check(root), [])


if __name__ == "__main__":
    unittest.main()

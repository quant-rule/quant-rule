#!/usr/bin/env python3
"""Unit tests for the port-parity checker."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_port_parity

TARGET = "crates/example/src/port.rs"
TEST = "crates/example/tests/port_parity.rs"
FIXTURE = "docs/port-parity/PORT-001.golden.json"


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def fixture(source: str, **overrides: object) -> dict:
    value = {
        "schema": check_port_parity.FIXTURE_SCHEMA,
        "id": "PORT-001",
        "source": source,
        "captured": "2026-11-14",
        "tolerance": 1e-9,
        "cases": [
            {
                "name": "one",
                "input": {"x": 1},
                "expected": {"y": 2},
            }
        ],
    }
    value.update(overrides)
    return value


def tree(root: Path, *, ports: list | None = None, **fixture_overrides: object) -> None:
    source = "tools/retired.py at 0123456"
    target = root / TARGET
    test = root / TEST
    write(target, f"// port-parity: PORT-001\npub fn ported() {{}}\n")
    write(
        test,
        f'// port-parity test: PORT-001\nconst FIXTURE: &str = "{FIXTURE}";\n',
    )
    write(root / FIXTURE, json.dumps(fixture(source, **fixture_overrides)))
    value = {
        "schema": check_port_parity.REGISTRY_SCHEMA,
        "ports": [
            {
                "id": "PORT-001",
                "source": source,
                "target": TARGET,
                "test": TEST,
                "fixture": FIXTURE,
                "status": "active",
            }
        ]
        if ports is None
        else ports,
    }
    write(root / check_port_parity.REGISTRY, json.dumps(value))


class PortParityCheckerTests(unittest.TestCase):
    def test_empty_registry_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, ports=[])
            self.assertEqual(check_port_parity.check(root), [])

    def test_registered_port_with_evidence_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            self.assertEqual(check_port_parity.check(root), [])

    def test_missing_fixture_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            (root / FIXTURE).unlink()
            problems = check_port_parity.check(root)
            self.assertTrue(any("fixture" in problem and "missing" in problem for problem in problems))

    def test_loose_tolerance_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, tolerance=1e-8)
            problems = check_port_parity.check(root)
            self.assertTrue(any("looser" in problem for problem in problems))

    def test_missing_target_marker_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            write(root / TARGET, "pub fn ported() {}\n")
            problems = check_port_parity.check(root)
            self.assertTrue(any("port-parity: PORT-001" in problem for problem in problems))

    def test_test_must_name_the_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            write(root / TEST, "// port-parity test: PORT-001\n")
            problems = check_port_parity.check(root)
            self.assertTrue(any("must name" in problem for problem in problems))

    def test_duplicate_id_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            registry = json.loads((root / check_port_parity.REGISTRY).read_text())
            registry["ports"].append(dict(registry["ports"][0]))
            write(root / check_port_parity.REGISTRY, json.dumps(registry))
            problems = check_port_parity.check(root)
            self.assertTrue(any("duplicate" in problem for problem in problems))

    def test_fixture_id_must_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, id="PORT-002")
            problems = check_port_parity.check(root)
            self.assertTrue(any("fixture id" in problem for problem in problems))

    def test_path_traversal_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            registry = json.loads((root / check_port_parity.REGISTRY).read_text())
            registry["ports"][0]["target"] = "../outside.rs"
            write(root / check_port_parity.REGISTRY, json.dumps(registry))
            problems = check_port_parity.check(root)
            self.assertTrue(any("inside the repository" in problem for problem in problems))

    def test_invalid_registry_json_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            write(root / check_port_parity.REGISTRY, "{not json")
            problems = check_port_parity.check(root)
            self.assertTrue(any("not valid JSON" in problem for problem in problems))

    def test_non_string_status_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            registry = json.loads((root / check_port_parity.REGISTRY).read_text())
            registry["ports"][0]["status"] = ["active"]
            write(root / check_port_parity.REGISTRY, json.dumps(registry))
            problems = check_port_parity.check(root)
            self.assertTrue(any("status must be" in problem for problem in problems))

    def test_non_string_fixture_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root)
            registry = json.loads((root / check_port_parity.REGISTRY).read_text())
            registry["ports"][0]["fixture"] = 123
            write(root / check_port_parity.REGISTRY, json.dumps(registry))
            problems = check_port_parity.check(root)
            self.assertTrue(any("fixture must be" in problem for problem in problems))

    def test_nan_tolerance_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree(root, tolerance=float("nan"))
            problems = check_port_parity.check(root)
            self.assertTrue(any("positive number" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()

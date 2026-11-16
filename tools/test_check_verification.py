#!/usr/bin/env python3
"""Unit tests for the verification manifest checker.

The required-class rule is the only gate that enforces the model-layer evidence
contract, so the tests prove an ``n/a`` scope cannot satisfy it.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_verification

TEST_NAME = "ema_row_contract"
RUST_FUNCTIONS = {TEST_NAME}
N_A = {"scope": "n/a", "tests": []}


def write(path: Path, document: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document), encoding="utf-8")


def make_manifest(overrides: dict[str, object] | None = None) -> dict[str, object]:
    evidence = {
        name: {"scope": "row", "tests": [TEST_NAME]} for name in check_verification.CLASSES
    }
    for name, item in (overrides or {}).items():
        evidence[name] = item
    return {
        "schema": "quantrule.verification/v1",
        "indicators": [{"id": "ema", "kind": "indicator", "evidence": evidence}],
        "events": [],
    }


class VerificationCheckerTests(unittest.TestCase):
    def check(self, overrides: dict[str, object] | None = None) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "registry" / "indicators.json",
                {"indicators": [{"id": "ema", "status": "active"}], "events": []},
            )
            write(
                root / "docs" / "registry" / "verification.json",
                make_manifest(overrides),
            )
            return check_verification.check(root, RUST_FUNCTIONS)

    def test_complete_manifest_passes(self) -> None:
        self.assertEqual(self.check(), [])

    def test_required_classes_cannot_be_not_applicable(self) -> None:
        for name in check_verification.REQUIRED_CLASSES:
            with self.subTest(name=name):
                problems = self.check({name: dict(N_A)})
                self.assertTrue(
                    any(f"{name} evidence cannot be n/a" in problem for problem in problems),
                    problems,
                )

    def test_optional_class_may_be_not_applicable(self) -> None:
        self.assertEqual(self.check({"availability": dict(N_A)}), [])

    def test_not_applicable_must_not_carry_tests(self) -> None:
        problems = self.check(
            {"availability": {"scope": "n/a", "tests": [TEST_NAME]}}
        )
        self.assertTrue(any("n/a must not carry tests" in problem for problem in problems))

    def test_missing_class_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "registry" / "indicators.json",
                {"indicators": [{"id": "ema", "status": "active"}], "events": []},
            )
            document = make_manifest()
            del document["indicators"][0]["evidence"]["edge"]
            write(root / "docs" / "registry" / "verification.json", document)
            problems = check_verification.check(root, RUST_FUNCTIONS)
            self.assertTrue(
                any("missing evidence class edge" in problem for problem in problems)
            )


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Keep the checker and checker-test lists honest.

A new checker without a dedicated test must state why in this file. The gate
then fails on a silent gap instead of letting the checker list drift.
"""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHECKERS = ROOT / "tools" / "checkers.txt"
TESTS = ROOT / "tools" / "checker_tests.txt"

# Checkers whose unit test would only restate the repository scan. The reason
# is the boundary: they read the whole tree, and the gate output names every
# file they examined.
EXEMPT = {
    "check_doc_length": "the checker reports every current document and its cap",
    "check_links": "the checker reports every broken relative link in the tree",
    "check_registry": "the registry invariants are checked by check_validation and check_verification_matrix",
    "check_terms": "the term gate is delegated to tools/improvement/cleanup.py",
    "check_validation": "the claims are data in the registry and the checker reports each missing claim",
    "check_verification_matrix": "the matrix is data and the checker reports every missing code",
}


def names(path: Path) -> list[str]:
    return [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


class CheckerRegistryTests(unittest.TestCase):
    def test_every_checker_has_a_test_or_a_documented_reason(self) -> None:
        checkers = names(CHECKERS)
        tests = set(names(TESTS))
        for checker in checkers:
            if f"test_{checker}" in tests:
                continue
            self.assertIn(checker, EXEMPT, f"{checker} has no test and no exemption")

    def test_every_exemption_is_a_checker(self) -> None:
        checkers = set(names(CHECKERS))
        for checker in EXEMPT:
            self.assertIn(checker, checkers)

    def test_no_checker_or_test_is_listed_twice(self) -> None:
        for path in (CHECKERS, TESTS):
            rows = names(path)
            self.assertEqual(len(rows), len(set(rows)), f"{path.name} has a duplicate")

    def test_every_listed_test_exists(self) -> None:
        for module in names(TESTS):
            self.assertTrue((ROOT / "tools" / f"{module}.py").is_file(), module)


if __name__ == "__main__":
    unittest.main()

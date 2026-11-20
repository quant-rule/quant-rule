#!/usr/bin/env python3
"""Unit tests for the generated-client and Rust label check."""

from __future__ import annotations

import unittest

from check_labels import RUST, rust_problems
from generate_labels import load, REGISTRY


class LabelCheckTests(unittest.TestCase):
    def test_the_tree_matches_the_registry(self) -> None:
        self.assertEqual(rust_problems(RUST.read_text(encoding="utf-8"), load(REGISTRY)), [])

    def test_a_changed_event_word_is_reported(self) -> None:
        source = RUST.read_text(encoding="utf-8").replace(
            '(1, 1) => "test/retest"', '(1, 1) => "test"'
        )
        problems = rust_problems(source, load(REGISTRY))
        self.assertTrue(
            any("event_subtype_label(1, 1)" in problem for problem in problems),
            problems,
        )

    def test_a_missing_source_is_reported(self) -> None:
        source = RUST.read_text(encoding="utf-8").replace('19 => "VWAP",\n', "")
        problems = rust_problems(source, load(REGISTRY))
        self.assertTrue(any("source_label19 is missing" in problem for problem in problems), problems)


if __name__ == "__main__":
    unittest.main()

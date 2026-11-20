#!/usr/bin/env python3
"""Unit tests for the event-emitter checker."""

from __future__ import annotations

import unittest

import check_event_emitters


class EventEmitterCheckerTests(unittest.TestCase):
    def test_test_modules_are_not_production_sources(self) -> None:
        names = {path.name for path in check_event_emitters.production_paths()}
        self.assertTrue(names, "the events tree has no production source")
        self.assertFalse(
            any(name.endswith("_tests.rs") for name in names),
            names,
        )
        self.assertNotIn("tests.rs", names)
        self.assertIn("signals.rs", names)


if __name__ == "__main__":
    unittest.main()

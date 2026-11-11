#!/usr/bin/env python3
"""Tests for the persistent local data-root resolver."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from data_root import (
    DEFAULT_DIRECTORY,
    DataRootError,
    current_worktree,
    default_root,
    guard_root,
    is_inside,
    resolve_root,
    substitute,
)


class DataRootTest(unittest.TestCase):
    def test_default_is_outside_the_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {"HOME": directory}, clear=False):
                with patch.dict(os.environ, {"QUANTRULE_DATA_ROOT": ""}, clear=False):
                    root = default_root()
                    self.assertEqual(root, Path(directory) / DEFAULT_DIRECTORY)
                    self.assertFalse(is_inside(root, current_worktree()))

    def test_environment_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {"QUANTRULE_DATA_ROOT": directory}, clear=False):
                self.assertEqual(resolve_root(), Path(directory).resolve())

    def test_explicit_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(resolve_root(directory), Path(directory).resolve())

    def test_substitute_placeholder(self) -> None:
        root = Path("/tmp/quant-rule-data")
        self.assertEqual(
            substitute("{data_root}/simulation/report", root),
            "/tmp/quant-rule-data/simulation/report",
        )
        self.assertEqual(substitute("/plain/path", root), "/plain/path")

    def test_guard_refuses_the_current_worktree(self) -> None:
        inside = current_worktree() / "data" / "simulation"
        with self.assertRaises(DataRootError):
            guard_root(inside, allow_worktree=False)
        guard_root(inside, allow_worktree=True)

    def test_is_inside(self) -> None:
        self.assertTrue(is_inside(Path("/tmp/root/a"), Path("/tmp/root")))
        self.assertFalse(is_inside(Path("/tmp/other/a"), Path("/tmp/root")))


if __name__ == "__main__":
    unittest.main()

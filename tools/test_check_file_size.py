#!/usr/bin/env python3
"""Unit tests for the physical-line counter behind the file-size gate."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_file_size import lines_in


class FileSizeTests(unittest.TestCase):
    def test_a_trailing_newline_is_one_line(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.rs"
            path.write_text("one\ntwo\nthree\n", encoding="utf-8")
            self.assertEqual(lines_in(path), 3)

    def test_a_file_without_a_trailing_newline_is_counted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.rs"
            path.write_text("one\ntwo\nthree", encoding="utf-8")
            self.assertEqual(lines_in(path), 3)


if __name__ == "__main__":
    unittest.main()

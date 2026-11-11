#!/usr/bin/env python3
"""Tests for the simulation mark schema and round-trip."""

from __future__ import annotations

import csv
import tempfile
import unittest
from pathlib import Path

from marks import (
    MarkError,
    annotation_key,
    read_marks,
    validate_mark,
    validate_marks,
    write_marks,
)

CHART = {
    "file": "sim-w001.png",
    "bars": [
        [10, 1514764800, 101.0, 99.0],
        [11, 1514765700, 102.0, 100.0],
    ],
}


def chart_map() -> dict[str, dict]:
    return {CHART["file"]: CHART}


class MarksTest(unittest.TestCase):
    def test_validate_derives_time(self) -> None:
        row = validate_mark(
            {
                "chart": "sim-w001.png",
                "bar": "10",
                "action": "entry",
                "quality": "good",
                "side": "long",
                "note": "test",
            },
            chart_map(),
        )
        self.assertEqual(row["marked_time_utc"], "2018-01-01T00:00:00Z")
        self.assertEqual(row["note"], "test")

    def test_unknown_chart_is_rejected(self) -> None:
        with self.assertRaises(MarkError):
            validate_mark(
                {"chart": "absent.png", "bar": "10", "action": "entry", "quality": "good", "side": "long"},
                chart_map(),
            )

    def test_missing_bar_is_rejected(self) -> None:
        with self.assertRaises(MarkError):
            validate_mark(
                {"chart": "sim-w001.png", "bar": "99", "action": "entry", "quality": "good", "side": "long"},
                chart_map(),
            )

    def test_bad_side_and_note_are_rejected(self) -> None:
        with self.assertRaises(MarkError):
            validate_mark(
                {"chart": "sim-w001.png", "bar": "10", "action": "entry", "quality": "good", "side": "up"},
                chart_map(),
            )
        with self.assertRaises(MarkError):
            validate_mark(
                {
                    "chart": "sim-w001.png",
                    "bar": "10",
                    "action": "entry",
                    "quality": "good",
                    "side": "long",
                    "note": "x" * 501,
                },
                chart_map(),
            )

    def test_duplicate_identity_is_rejected(self) -> None:
        row = {
            "chart": "sim-w001.png",
            "bar": "10",
            "action": "entry",
            "quality": "good",
            "side": "long",
            "note": "",
        }
        with self.assertRaises(MarkError):
            validate_marks([row, row], [CHART])

    def test_write_read_round_trip_is_sorted(self) -> None:
        rows = [
            validate_mark(
                {"chart": "sim-w001.png", "bar": "11", "action": "exit", "quality": "good", "side": "long"},
                chart_map(),
            ),
            validate_mark(
                {"chart": "sim-w001.png", "bar": "10", "action": "entry", "quality": "good", "side": "long"},
                chart_map(),
            ),
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "marks.csv"
            write_marks(path, rows)
            loaded = read_marks(path)
        self.assertEqual([row["bar"] for row in loaded], ["10", "11"])
        self.assertEqual(annotation_key(loaded[0]), ("sim-w001.png", "10", "entry", "long"))


if __name__ == "__main__":
    unittest.main()

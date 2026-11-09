#!/usr/bin/env python3
"""Tests for the source coverage and cross-venue audit."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from audit_coverage import (
    audit_source,
    bucket_coverage,
    compare_sources,
    percentile,
    parse_timestamp,
)


def write_source(path: Path, rows: list[str]) -> None:
    path.write_text("Date,Open,High,Low,Close,Volume\n" + "\n".join(rows) + "\n", encoding="utf-8")


class TimestampTest(unittest.TestCase):
    def test_parses_the_source_shape(self) -> None:
        self.assertEqual(parse_timestamp("2023-01-01 00:00:00+00:00"), 1_672_531_200)

    def test_rejects_a_bad_shape(self) -> None:
        with self.assertRaises(Exception):
            parse_timestamp("not-a-time")


class BucketTest(unittest.TestCase):
    def test_full_bucket_is_counted(self) -> None:
        flags = bytearray(60)
        flags[:] = b"\x01" * 60
        result = bucket_coverage(flags, 15)
        self.assertEqual(result["full"], 4)
        self.assertEqual(result["coverage"], 1.0)

    def test_a_missing_minute_drops_the_bucket(self) -> None:
        flags = bytearray(60)
        flags[:] = b"\x01" * 60
        flags[7] = 0
        result = bucket_coverage(flags, 15)
        self.assertEqual(result["full"], 3)

    def test_percentile(self) -> None:
        self.assertEqual(percentile([0.0, 1.0], 0.5), 0.5)
        self.assertEqual(percentile([0.0, 1.0], 0.95), 0.95)


class AuditTest(unittest.TestCase):
    def test_audits_rows_duplicates_and_gaps(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.csv"
            write_source(
                path,
                [
                    "2023-01-01 00:00:00+00:00,1,1,1,1,0",
                    "2023-01-01 00:02:00+00:00,1,1,1,1,0",
                    "2023-01-01 00:02:00+00:00,1,1,1,1,0",
                ],
            )
            report = audit_source(path, "TEST")
            self.assertEqual(report["rows"], 3)
            self.assertEqual(report["duplicates"], 1)
            self.assertEqual(report["gaps"]["count"], 1)
            self.assertEqual(report["gaps"]["missing_minutes"], 1)
            self.assertEqual(report["flat_zero"], 3)

    def test_compares_aligned_closes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            left = Path(directory) / "left.csv"
            right = Path(directory) / "right.csv"
            write_source(
                left,
                [
                    "2023-01-01 00:00:00+00:00,10,11,9,10,1",
                    "2023-01-01 00:01:00+00:00,10,11,9,11,1",
                ],
            )
            write_source(
                right,
                [
                    "2023-01-01 00:00:00+00:00,10,11,9,10.5,1",
                    "2023-01-01 00:01:00+00:00,10,11,9,11,1",
                ],
            )
            start = parse_timestamp("2023-01-01 00:00:00+00:00")
            end = parse_timestamp("2023-01-01 00:01:00+00:00")
            report = compare_sources(left, right, start, end)
            self.assertEqual(report["overlap_rows"], 2)
            self.assertEqual(report["ohlc_mismatch_rows"], 1)
            self.assertAlmostEqual(report["close_difference"]["max"], 0.5)
            self.assertAlmostEqual(report["basis_fraction"]["median"], -0.023809523809523808)


if __name__ == "__main__":
    unittest.main()

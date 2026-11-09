#!/usr/bin/env python3
"""Tests for the Binance monthly kline importer."""

from __future__ import annotations

import json
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

from binance_klines import (
    BinanceImportError,
    checksum_from_text,
    format_number,
    format_timestamp,
    import_range,
    kline_timestamp,
    month_files,
    parse_month,
    rows_from_zip,
    sha256_file,
    validate_row,
    write_manifest,
)


def zip_with_csv(path: Path, text: str) -> None:
    """Write a one-member zip containing the given CSV text."""
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("BTCUSDT-1m-2020-01.csv", text)


class MonthRangeTest(unittest.TestCase):
    def test_inclusive_range(self) -> None:
        files = month_files("BTCUSDT", "2023-01", "2023-03")
        self.assertEqual([(item.year, item.month) for item in files], [(2023, 1), (2023, 2), (2023, 3)])
        self.assertTrue(files[0].url.endswith("/BTCUSDT/1m/BTCUSDT-1m-2023-01.zip"))
        self.assertTrue(files[0].checksum_url.endswith(".zip.CHECKSUM"))

    def test_range_crosses_a_year(self) -> None:
        files = month_files("BTCUSDT", "2024-12", "2025-02")
        self.assertEqual([(item.year, item.month) for item in files], [(2024, 12), (2025, 1), (2025, 2)])

    def test_reversed_range_is_rejected(self) -> None:
        with self.assertRaises(BinanceImportError):
            month_files("BTCUSDT", "2024-01", "2023-12")

    def test_invalid_month_is_rejected(self) -> None:
        with self.assertRaises(BinanceImportError):
            parse_month("2024-13")


class ChecksumTest(unittest.TestCase):
    def test_checksum_without_trailing_newline(self) -> None:
        digest = "a" * 64
        self.assertEqual(checksum_from_text(f"{digest}  file.zip"), digest)

    def test_checksum_with_newline_and_uppercase(self) -> None:
        digest = "abcdef" * 10 + "abcd"
        self.assertEqual(checksum_from_text(f"{digest.upper()}  file.zip\n"), digest)

    def test_checksum_rejects_a_short_value(self) -> None:
        with self.assertRaises(BinanceImportError):
            checksum_from_text("not-a-checksum")

    def test_sha256_of_a_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "payload"
            path.write_bytes(b"quantrule")
            self.assertEqual(len(sha256_file(path)), 64)


class TimestampTest(unittest.TestCase):
    def test_milliseconds(self) -> None:
        self.assertEqual(kline_timestamp("1672531200000"), (1672531200, True))

    def test_microseconds(self) -> None:
        self.assertEqual(kline_timestamp("1735689600000000"), (1735689600, True))

    def test_misaligned_minute_is_floored_and_recorded(self) -> None:
        self.assertEqual(kline_timestamp("1672531260001"), (1672531260, False))

    def test_format_is_the_source_shape(self) -> None:
        self.assertEqual(format_timestamp(0), "1970-01-01 00:00:00+00:00")

    def test_integral_float_keeps_a_decimal_point(self) -> None:
        self.assertEqual(format_number(1.0), "1.0")
        self.assertEqual(format_number(1.5), "1.5")


class RowTest(unittest.TestCase):
    def test_valid_row(self) -> None:
        row = ["1672531200000", "16541.77", "16544.76", "16538.45", "16543.67", "83.08143"]
        self.assertEqual(
            validate_row(row, "row"),
            (1672531200, 16541.77, 16544.76, 16538.45, 16543.67, 83.08143, True),
        )

    def test_high_below_close_is_rejected(self) -> None:
        row = ["1672531200000", "10", "9", "8", "10.5", "1"]
        with self.assertRaises(BinanceImportError):
            validate_row(row, "row")

    def test_negative_volume_is_rejected(self) -> None:
        row = ["1672531200000", "10", "11", "9", "10", "-1"]
        with self.assertRaises(BinanceImportError):
            validate_row(row, "row")

    def test_non_numeric_price_is_rejected(self) -> None:
        row = ["1672531200000", "x", "11", "9", "10", "1"]
        with self.assertRaises(BinanceImportError):
            validate_row(row, "row")


class ZipTest(unittest.TestCase):
    def test_rows_without_a_header(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "month.zip"
            zip_with_csv(
                path,
                "1672531200000,10,11,9,10.5,1\n"
                "1672531260000,10.5,12,10,11,2\n",
            )
            rows = list(rows_from_zip(path))
            self.assertEqual(len(rows), 2)
            self.assertEqual(rows[0][0], 1672531200)
            self.assertEqual(rows[1][1], 10.5)

    def test_header_is_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "month.zip"
            zip_with_csv(path, "open_time,open,high,low,close,volume\n1672531200000,10,11,9,10.5,1\n")
            rows = list(rows_from_zip(path))
            self.assertEqual(len(rows), 1)


class ImportTest(unittest.TestCase):
    def test_import_writes_the_source_header_and_is_repeatable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_zip = root / "BTCUSDT-1m-2020-01.zip"
            zip_with_csv(
                source_zip,
                "1577836800000,7195.24,7198.0,7190.0,7195.0,1.5\n"
                "1577836860000,7195.0,7200.0,7194.0,7199.0,2.5\n",
            )
            digest = sha256_file(source_zip)
            out = root / "out.csv"
            manifest = root / "manifest.json"
            with mock.patch("binance_klines.ensure_month", return_value=(source_zip, digest)):
                first = import_range("BTCUSDT", "2020-01", "2020-01", out, root)
                second = import_range("BTCUSDT", "2020-01", "2020-01", out, root)
            self.assertEqual(first.sha256, second.sha256)
            self.assertEqual(out.read_text(encoding="utf-8").splitlines()[0], "Date,Open,High,Low,Close,Volume")
            write_manifest(first, out, manifest, "binance-test-v1")
            document = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(document["rows"], 2)
            self.assertEqual(document["data_version"], "binance-test-v1")

    def test_invalid_row_leaves_no_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source_zip = root / "BTCUSDT-1m-2020-01.zip"
            zip_with_csv(source_zip, "1577836800000,10,9,8,10.5,1\n")
            digest = sha256_file(source_zip)
            out = root / "out.csv"
            with mock.patch("binance_klines.ensure_month", return_value=(source_zip, digest)):
                with self.assertRaises(BinanceImportError):
                    import_range("BTCUSDT", "2020-01", "2020-01", out, root)
            self.assertFalse(out.exists())
            self.assertFalse(out.with_name(out.name + ".tmp").exists())


if __name__ == "__main__":
    unittest.main()

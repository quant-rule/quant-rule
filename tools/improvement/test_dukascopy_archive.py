#!/usr/bin/env python3
"""Unit tests for the Dukascopy archive acquisition tool."""

from __future__ import annotations

import csv
import lzma
import tempfile
import unittest
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

from dukascopy_archive import (
    Candle,
    RECORD,
    archive_url,
    build_year,
    decode_candles,
    fetch_day,
    format_volume,
    normalize,
    summarize,
    write_csv,
)


def payload(records: list[tuple[int, int, int, int, int, float]]) -> bytes:
    return lzma.compress(b"".join(RECORD.pack(*record) for record in records))


class ArchiveTests(unittest.TestCase):
    def test_url_uses_zero_based_month(self) -> None:
        self.assertEqual(
            archive_url("BTCUSD", date(2023, 1, 2)),
            "https://datafeed.dukascopy.com/datafeed/BTCUSD/2023/00/02/BID_candles_min_1.bi5",
        )
        self.assertEqual(
            archive_url("BTCUSD", date(2023, 12, 31)),
            "https://datafeed.dukascopy.com/datafeed/BTCUSD/2023/11/31/BID_candles_min_1.bi5",
        )

    def test_decode_orders_the_fields_and_scales_prices(self) -> None:
        blob = payload([(0, 165754, 165773, 165754, 165773, 0.000437), (60, 165774, 165809, 165770, 165809, 0.000427)])
        candles = decode_candles(blob, date(2023, 1, 2))
        self.assertEqual(len(candles), 2)
        first = candles[0]
        self.assertEqual(first.timestamp, datetime(2023, 1, 2, tzinfo=timezone.utc))
        self.assertEqual((first.open, first.high, first.low, first.close), (16575.4, 16577.3, 16575.4, 16577.3))
        self.assertAlmostEqual(first.volume, 0.000437)
        self.assertEqual(candles[1].timestamp, datetime(2023, 1, 2, 0, 1, tzinfo=timezone.utc))

    def test_decode_rejects_a_truncated_record(self) -> None:
        blob = lzma.compress(b"\x00" * 23)
        with self.assertRaises(ValueError):
            decode_candles(blob, date(2023, 1, 2))

    def test_normalize_sorts_and_keeps_the_first_duplicate(self) -> None:
        later = Candle(datetime(2023, 1, 2, 0, 1, tzinfo=timezone.utc), 2, 2, 2, 2, 2)
        first = Candle(datetime(2023, 1, 2, tzinfo=timezone.utc), 1, 1, 1, 1, 1)
        duplicate = Candle(datetime(2023, 1, 2, tzinfo=timezone.utc), 9, 9, 9, 9, 9)
        self.assertEqual(normalize([later, first, duplicate]), [first, later])

    def test_volume_is_trimmed_not_rounded_to_zero(self) -> None:
        self.assertEqual(format_volume(0.000049), "0.000049")
        self.assertEqual(format_volume(0.0), "0")
        self.assertEqual(format_volume(0.000437), "0.000437")

    def test_summarize_separates_row_coverage_from_flat_placeholders(self) -> None:
        start = datetime(2023, 1, 2, tzinfo=timezone.utc)
        candles = [Candle(start + timedelta(minutes=index), 1, 1, 1, 1, 0) for index in range(2)]
        candles += [Candle(start + timedelta(minutes=index), 2, 2.1, 1.9, 2, 1) for index in range(2, 60)]
        summary = summarize(candles)
        self.assertEqual(summary["rows"], 60)
        self.assertEqual(summary["full_hours"], 1)
        self.assertEqual(summary["coverage_share"], 1.0)
        self.assertEqual(summary["flat_zero_bars"], 2)
        self.assertEqual(summary["longest_flat_run"], 2)

    def test_write_csv_matches_the_repository_header(self) -> None:
        candle = Candle(datetime(2023, 1, 2, tzinfo=timezone.utc), 16575.4, 16577.3, 16575.4, 16577.3, 0.000437)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "out.csv"
            write_csv([candle], path)
            with path.open(newline="") as handle:
                rows = list(csv.reader(handle))
        self.assertEqual(rows[0], ["Date", "Open", "High", "Low", "Close", "Volume"])
        self.assertEqual(rows[1], ["2023-01-02 00:00:00+00:00", "16575.4", "16577.3", "16575.4", "16577.3", "0.000437"])

    def test_fetch_day_reuses_a_cached_file_without_calling_the_network(self) -> None:
        calls = []

        def fetch(_url: str) -> bytes:
            calls.append(_url)
            return b"should not be used"

        with tempfile.TemporaryDirectory() as directory:
            raw = Path(directory)
            (raw / "2023-01-02.bi5").write_bytes(b"cached")
            day, status = fetch_day("BTCUSD", date(2023, 1, 2), raw, fetch=fetch)
        self.assertEqual((day, status), (date(2023, 1, 2), "cached"))
        self.assertEqual(calls, [])

    def test_build_year_writes_a_deterministic_csv(self) -> None:
        day_blob = payload([(0, 100, 110, 90, 105, 0.5)])

        def fetch(_url: str) -> bytes:
            return day_blob

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            summary = build_year("BTCUSD", 2023, root / "out.csv", root / "raw", workers=2, fetch=fetch)
            with (root / "out.csv").open(newline="") as handle:
                rows = list(csv.DictReader(handle))
        self.assertEqual(summary["quality"]["rows"], 365)
        self.assertEqual(len(rows), 365)
        self.assertEqual(rows[0]["Date"], "2023-01-01 00:00:00+00:00")
        self.assertEqual(rows[-1]["Date"], "2023-12-31 00:00:00+00:00")


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Tests for the simulation bundle loader."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

from bundle import (
    BundleError,
    M13_EVENT_CODES,
    load_bundle,
    load_catalog,
    validate_m13,
)

FIXTURE = Path(__file__).parent / "fixtures" / "mini"
REGISTRY = Path(__file__).resolve().parents[2] / "docs" / "registry" / "indicators.json"


class BundleTest(unittest.TestCase):
    def test_loads_fixture(self) -> None:
        bundle = load_bundle(FIXTURE)
        self.assertEqual(len(bundle.bars), 400)
        self.assertEqual(len(bundle.events), 6)
        self.assertEqual(bundle.data_version, "fixture-m1.3-v1")
        self.assertEqual(bundle.model_version, "fixture")
        self.assertEqual(bundle.bars[0].bar, 0)
        self.assertEqual(bundle.bars[-1].bar, 399)

    def test_m13_validation_passes(self) -> None:
        bundle = load_bundle(FIXTURE)
        validate_m13(bundle)
        self.assertTrue(M13_EVENT_CODES & {event.code for event in bundle.events})

    def test_events_are_sorted_by_available_bar(self) -> None:
        bundle = load_bundle(FIXTURE)
        available = [event.available_bar for event in bundle.events]
        self.assertEqual(available, sorted(available))

    def test_catalog_has_names(self) -> None:
        catalog = load_catalog(REGISTRY)
        self.assertEqual(catalog[149], "level.fib_test")
        self.assertEqual(catalog[150], "level.band_test")

    def test_missing_column_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with (root / "readings.csv").open("w", newline="") as handle:
                writer = csv.DictWriter(handle, fieldnames=["timeframe_secs", "bar"])
                writer.writeheader()
                writer.writerow({"timeframe_secs": 900, "bar": 0})
            (root / "events.csv").write_text("", encoding="utf-8")
            with self.assertRaises(BundleError):
                load_bundle(root)

    def test_missing_m13_columns_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = list(csv.DictReader((FIXTURE / "readings.csv").open(encoding="utf-8")))
            for row in rows:
                for column in ("fib_nearest_atr", "band_upper", "band_lower", "band_position"):
                    row.pop(column, None)
            with (root / "readings.csv").open("w", newline="") as handle:
                writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
                writer.writeheader()
                writer.writerows(rows)
            (root / "events.csv").write_text(
                (FIXTURE / "events.csv").read_text(encoding="utf-8"), encoding="utf-8"
            )
            bundle = load_bundle(root)
            with self.assertRaises(BundleError):
                validate_m13(bundle)

    def test_date_filter(self) -> None:
        bundle = load_bundle(FIXTURE, from_ts=1514764800 + 900 * 100)
        self.assertEqual(bundle.bars[0].bar, 100)
        self.assertEqual(len(bundle.bars), 300)


if __name__ == "__main__":
    unittest.main()

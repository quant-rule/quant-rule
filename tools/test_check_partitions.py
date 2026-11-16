#!/usr/bin/env python3
"""Unit tests for the partition checker."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_partitions


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def make_partitions(root: Path, instruments: list[str] | None = None) -> None:
    document = {
        "schema": "quantrule.partitions/v1",
        "timezone": "UTC",
        "instruments": instruments or ["BTCUSD", "XAUUSD"],
        "aliases": {"BTC": "BTCUSD", "XAU": "XAUUSD"},
        "excluded_windows": [],
        "partitions": {
            "model_dev": {
                "window": {"from": "2018-01-01", "to": "2020-12-31"},
                "use": "Model development",
                "instruments": ["BTCUSD", "XAUUSD"],
            }
        },
    }
    write(root / "data" / "partitions.json", json.dumps(document))


class PartitionCheckerTests(unittest.TestCase):
    def test_valid_partitions_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            write(root / "config" / "example" / "run.json", json.dumps({"instrument": "BTC"}))
            self.assertEqual(check_partitions.check(root), [])

    def test_undeclared_partition_instrument_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["partitions"]["model_dev"]["instruments"].append("EURUSD")
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("undeclared instrument" in problem for problem in problems))

    def test_alias_target_must_be_canonical(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["aliases"]["SP500"] = "ESANDP500"
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("does not resolve" in problem for problem in problems))

    def test_config_instrument_must_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            write(root / "config" / "example" / "run.json", json.dumps({"instrument": "EURUSD"}))
            problems = check_partitions.check(root)
            self.assertTrue(any("is not declared" in problem for problem in problems))

    def test_nested_config_instrument_must_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            write(
                root / "config" / "example" / "strategy.json",
                json.dumps({"legs": [{"instrument": "EURUSD"}]}),
            )
            problems = check_partitions.check(root)
            self.assertTrue(
                any("strategy.json:legs[0].instrument" in problem for problem in problems),
                problems,
            )

    def test_nested_declared_instrument_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            write(
                root / "config" / "example" / "strategy.json",
                json.dumps({"legs": [{"instrument": "BTC"}]}),
            )
            self.assertEqual(check_partitions.check(root), [])

    def test_excluded_window_without_reason_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["excluded_windows"] = [{"from": "2023-01-01", "to": "2023-12-31"}]
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("has no reason" in problem for problem in problems))

    def test_partition_overlapping_an_excluded_window_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["excluded_windows"] = [
                {"from": "2023-01-01", "to": "2023-12-31", "reason": "Reset"}
            ]
            document["partitions"]["model_dev"]["window"]["to"] = "2023-06-30"
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("overlaps excluded window" in problem for problem in problems))

    def test_overlapping_excluded_windows_are_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["excluded_windows"] = [
                {"from": "2023-01-01", "to": "2023-12-31", "reason": "Reset"},
                {"from": "2023-06-01", "to": "2023-07-31", "reason": "Halt"},
            ]
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("excluded_windows overlap" in problem for problem in problems))

    def test_out_of_order_excluded_windows_are_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_partitions(root)
            document = json.loads((root / "data" / "partitions.json").read_text())
            document["excluded_windows"] = [
                {"from": "2025-01-01", "to": "2025-01-31", "reason": "Halt"},
                {"from": "2023-01-01", "to": "2023-12-31", "reason": "Reset"},
            ]
            write(root / "data" / "partitions.json", json.dumps(document))
            problems = check_partitions.check(root)
            self.assertTrue(any("is out of order" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()

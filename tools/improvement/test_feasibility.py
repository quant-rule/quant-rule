#!/usr/bin/env python3
"""Unit tests for the strategy feasibility preflight."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

import feasibility


def write_candles(path: Path, count: int, range_bps: float = 10.0) -> None:
    start = datetime(2020, 1, 1, tzinfo=timezone.utc)
    price = 100.0
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["Date", "Open", "High", "Low", "Close", "Volume"])
        for index in range(count):
            time = start + timedelta(minutes=index)
            half = price * range_bps / 20_000.0
            writer.writerow(
                [
                    time.isoformat(),
                    price,
                    price + half,
                    price - half,
                    price,
                    1.0,
                ]
            )


def data_spec(path: Path, available: tuple[str, ...] = ("1m", "5m", "15m", "1h")) -> feasibility.DataSpec:
    return feasibility.DataSpec(
        path=path,
        instrument="TESTUSD",
        source_timeframe="1m",
        source_secs=60,
        available_timeframes=available,
        start=None,
        end=None,
    )


def strategy(
    execution: str = "1m",
    execution_secs: int = 60,
    stop_atr: float = 4.0,
    target_atr: float = 1.0,
    context: tuple[str, ...] = (),
    context_secs: tuple[int, ...] = (),
) -> feasibility.StrategyRisk:
    return feasibility.StrategyRisk(
        strategy_id="TEST",
        engine="event_composite",
        execution_timeframe=execution,
        execution_secs=execution_secs,
        context_timeframes=context,
        context_secs=context_secs,
        stop_atr=stop_atr,
        target_atr=target_atr,
    )


class FeasibilityTests(unittest.TestCase):
    def test_resample_aggregates_ohlcv(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candles.csv"
            write_candles(path, 10)
            bars, truncated = feasibility.read_bars(data_spec(path), 300, 100)
        self.assertFalse(truncated)
        self.assertEqual(len(bars), 2)
        self.assertEqual(bars[0].open, 100.0)
        self.assertEqual(bars[0].close, 100.0)
        self.assertEqual(bars[0].volume, 5.0)
        self.assertGreater(bars[0].high, bars[0].low)

    def test_four_to_one_fade_fails_the_breakeven_gate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candles.csv"
            write_candles(path, 2_000, range_bps=15.0)
            report = feasibility.evaluate(
                strategy(stop_atr=4.0, target_atr=1.0), data_spec(path), 6.0
            )
        self.assertEqual(report["verdict"], "fail")
        self.assertIn("breakeven_above_50pct", report["reasons"])
        self.assertGreater(report["breakeven_win_rate"], 0.50)

    def test_one_to_two_on_five_minutes_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candles.csv"
            write_candles(path, 4_000, range_bps=15.0)
            report = feasibility.evaluate(
                strategy(
                    execution="5m",
                    execution_secs=300,
                    stop_atr=1.0,
                    target_atr=2.0,
                ),
                data_spec(path),
                6.0,
            )
        self.assertEqual(report["verdict"], "pass", report["reasons"])
        self.assertLessEqual(report["breakeven_win_rate"], 0.50)
        self.assertGreater(report["net_target_atr"], 0.0)

    def test_target_below_cost_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candles.csv"
            write_candles(path, 2_000, range_bps=0.2)
            report = feasibility.evaluate(
                strategy(stop_atr=4.0, target_atr=1.0), data_spec(path), 6.0
            )
        self.assertEqual(report["verdict"], "fail")
        self.assertIn("target_below_cost", report["reasons"])
        self.assertIsNone(report["breakeven_win_rate"])

    def test_timeframe_above_one_hour_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candles.csv"
            write_candles(path, 30_000, range_bps=15.0)
            report = feasibility.evaluate(
                strategy(
                    execution="4h",
                    execution_secs=14_400,
                    stop_atr=1.0,
                    target_atr=2.0,
                ),
                data_spec(path, available=("1m", "4h")),
                6.0,
            )
        self.assertEqual(report["verdict"], "fail")
        self.assertIn("timeframe_above_1h", report["reasons"])

    def test_load_strategy_and_data_configs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            strategy_path = root / "strategy.json"
            strategy_path.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.strategy/v1",
                        "id": "S-TEST",
                        "engine": "event_composite",
                        "execution_timeframe": "5m",
                        "context_timeframes": ["15m", "1h"],
                        "risk": {"stop_atr_multiple": 1.0, "reward_to_risk": 2.0},
                    }
                ),
                encoding="utf-8",
            )
            loaded_strategy = feasibility.load_strategy(strategy_path)
            self.assertEqual(loaded_strategy.execution_secs, 300)
            self.assertEqual(loaded_strategy.target_atr, 2.0)
            self.assertEqual(loaded_strategy.context_secs, (900, 3_600))

            data_path = root / "data.json"
            data_path.write_text(
                json.dumps(
                    {
                        "schema": "quantrule.data/v3",
                        "source": {"kind": "csv", "path": "candles.csv"},
                        "source_timeframe": "1m",
                        "available_timeframes": ["1m", "5m", "15m", "1h"],
                        "instrument": "TESTUSD",
                        "from": "2020-01-01",
                        "to": "2020-01-02",
                    }
                ),
                encoding="utf-8",
            )
            loaded_data = feasibility.load_data(data_path)
            self.assertEqual(loaded_data.path, (root / "candles.csv").resolve())
            self.assertEqual(loaded_data.instrument, "TESTUSD")

    def test_parse_timeframe(self) -> None:
        self.assertEqual(feasibility.parse_timeframe("1m", "$"), 60)
        self.assertEqual(feasibility.parse_timeframe("15m", "$"), 900)
        self.assertEqual(feasibility.parse_timeframe("1h", "$"), 3_600)
        with self.assertRaises(feasibility.FeasibilityError):
            feasibility.parse_timeframe("4x", "$")
        with self.assertRaises(feasibility.FeasibilityError):
            feasibility.parse_timeframe("0m", "$")


if __name__ == "__main__":
    unittest.main()

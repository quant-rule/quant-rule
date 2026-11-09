#!/usr/bin/env python3
"""Tests for the 15m trade-chart renderer."""

from __future__ import annotations

import csv
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from trade_charts import (
    MAX_CANDLES,
    Bar,
    Run,
    Trade,
    ema,
    find_stop_target,
    window_bounds,
)

PILLOW = True
try:
    import PIL  # noqa: F401
except ModuleNotFoundError:  # pragma: no cover
    PILLOW = False


def write_csv(path: Path, header: list[str], rows: list[list[object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(rows)


def make_run(root: Path) -> Path:
    run = root / "2018" / "breakonly-9bps" / "long"
    readings = []
    base = 1_600_000_000
    for bar in range(200):
        price = 10_000 + bar * 2
        readings.append([900, bar, base + bar * 900, price, price + 50, price - 50, price + 10, 100 + bar])
    write_csv(
        run / "backtest" / "model" / "readings.csv",
        ["timeframe_secs", "bar", "timestamp", "open", "high", "low", "close", "volume"],
        readings,
    )
    trades = [
        [1, "long", 1.0, 50, base + 50 * 900, 10_100, 60, base + 60 * 900, 10_300, 0, 0, 0, 0, 200.0, 0.02, 10, "target"],
        [2, "long", 1.0, 150, base + 150 * 900, 10_300, 170, base + 170 * 900, 10_200, 0, 0, 0, 0, -100.0, -0.01, 20, "stop"],
    ]
    write_csv(
        run / "backtest" / "portfolio" / "trades.csv",
        [
            "trade_id",
            "side",
            "quantity",
            "entry_bar",
            "entry_time",
            "entry_price",
            "exit_bar",
            "exit_time",
            "exit_price",
            "gross_pnl",
            "execution_fees",
            "commission",
            "funding",
            "net_pnl",
            "return_fraction",
            "hold_bars",
            "exit_reason",
        ],
        trades,
    )
    write_csv(
        run / "backtest" / "execution" / "orders.csv",
        ["order_id", "submitted_bar", "submitted_time", "side", "quantity", "reason", "trigger_kind", "trigger_price", "state"],
        [
            [0, 49, base + 49 * 900, "buy", 1.0, "entry", "market", "", "filled"],
            [1, 50, base + 50 * 900, "sell", 1.0, "take_profit", "target", 10_400, "filled"],
            [2, 149, base + 149 * 900, "buy", 1.0, "entry", "market", "", "filled"],
            [3, 150, base + 150 * 900, "sell", 1.0, "stop_loss", "stop", 9_900, "filled"],
        ],
    )
    return run


def run_for_test(root: Path) -> Run:
    make_run(root)
    return Run(
        path=root / "2018" / "breakonly-9bps" / "long",
        label="2018",
        bars=[
            Bar(
                bar=bar,
                timestamp=1_600_000_000 + bar * 900,
                open=10_000 + bar * 2,
                high=10_050 + bar * 2,
                low=9_950 + bar * 2,
                close=10_010 + bar * 2,
                volume=100 + bar,
            )
            for bar in range(200)
        ],
        trades=[
            Trade(1, "long", 50, 1_600_045_000, 10_100, 60, 1_600_054_000, 10_300, 200.0, 0.02, 10, "target"),
            Trade(2, "long", 150, 1_600_135_000, 10_300, 170, 1_600_153_000, 10_200, -100.0, -0.01, 20, "stop"),
        ],
        orders=[
            {"submitted_bar": "50", "reason": "take_profit", "trigger_price": "10400"},
            {"submitted_bar": "150", "reason": "stop_loss", "trigger_price": "9900"},
        ],
    )


class WindowTest(unittest.TestCase):
    def test_caps_at_one_thousand_and_contains_entry(self) -> None:
        start, end = window_bounds(5_000, 20_000)
        self.assertEqual(end - start, MAX_CANDLES)
        self.assertLessEqual(start, 5_000)
        self.assertGreater(end, 5_000)

    def test_short_series_uses_the_whole_series(self) -> None:
        self.assertEqual(window_bounds(50, 200), (0, 200))

    def test_entry_near_the_end_shifts_the_window(self) -> None:
        start, end = window_bounds(19_900, 20_000)
        self.assertEqual((start, end), (19_000, 20_000))


class EmaTest(unittest.TestCase):
    def test_ema_is_none_before_the_period(self) -> None:
        values = ema([1.0, 2.0, 3.0, 4.0], 3)
        self.assertEqual(values[:2], [None, None])
        self.assertIsNotNone(values[2])

    def test_ema_is_causal(self) -> None:
        full = ema([1.0, 2.0, 3.0, 4.0], 3)
        prefix = ema([1.0, 2.0, 3.0], 3)
        self.assertEqual(full[:3], prefix)


class StopTargetTest(unittest.TestCase):
    def test_finds_the_recorded_levels(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run = run_for_test(Path(directory))
            self.assertEqual(find_stop_target(run, run.trades[0]), (None, 10400.0))
            self.assertEqual(find_stop_target(run, run.trades[1]), (9900.0, None))


@unittest.skipUnless(PILLOW, "Pillow is required to render charts")
class RenderTest(unittest.TestCase):
    def test_renders_one_chart_per_trade_and_is_deterministic(self) -> None:
        from trade_charts import render

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = root / "artifacts"
            make_run(base)
            first = render(
                base,
                root / "first",
                "*/breakonly-9bps/long",
                "15m",
                MAX_CANDLES,
                "light",
                "BTCUSD 15m",
                "test control",
            )
            second = render(
                base,
                root / "second",
                "*/breakonly-9bps/long",
                "15m",
                MAX_CANDLES,
                "light",
                "BTCUSD 15m",
                "test control",
            )
            self.assertEqual(len(first), 2)
            self.assertEqual(len(second), 2)
            self.assertEqual([row["sha256"] for row in first], [row["sha256"] for row in second])
            for record in first:
                self.assertTrue((root / "first" / "charts" / record["file"]).is_file())
            manifest = json.loads((root / "first" / "charts" / "manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["schema"], "quantrule.trade-charts/v1")
            self.assertEqual(len(manifest["charts"]), 2)
            index = (root / "first" / "charts" / "index.csv").read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(index), 3)

    def test_rejects_a_cap_above_one_thousand(self) -> None:
        from trade_charts import render

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                render(root, root / "out", "*", "15m", 1001, "light", "BTCUSD 15m", "test")


if __name__ == "__main__":
    unittest.main()

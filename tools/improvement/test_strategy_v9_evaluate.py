#!/usr/bin/env python3
"""Unit tests for the v9 strategy matrix evaluator."""

from __future__ import annotations

import csv
import json
import tempfile
import unittest
from pathlib import Path

import strategy_v9_evaluate


def write_cell(
    run: Path,
    total_return: float,
    drawdown: float,
    trades: int,
    pnls: list[float],
    halted: bool | None = None,
) -> None:
    portfolio = run / "portfolio"
    portfolio.mkdir(parents=True)
    (portfolio / "report.json").write_text(
        json.dumps(
            {
                "summary": {
                    "total_return": total_return,
                    "max_drawdown": drawdown,
                    "trades": trades,
                    "halted": drawdown >= 0.10 if halted is None else halted,
                    "win_rate": 0.5,
                    "final_equity": 10_000.0 * (1.0 + total_return),
                    "commission": 1.0,
                    "execution_fees": 0.0,
                }
            }
        )
    )
    with (portfolio / "trades.csv").open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["net_pnl"])
        for pnl in pnls:
            writer.writerow([pnl])


class EvaluateTests(unittest.TestCase):
    def test_compounding_and_profit_factor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_cell(root / "btc-2019", 0.10, 0.05, 40, [10.0, -5.0])
            write_cell(root / "btc-2020", -0.05, 0.08, 35, [5.0, -10.0])
            cells = [
                {"instrument": "BTCUSD", "fold": 2019, "run": str(root / "btc-2019")},
                {"instrument": "BTCUSD", "fold": 2020, "run": str(root / "btc-2020")},
            ]
            args = strategy_v9_evaluate.parse_arguments(
                [
                    "--matrix",
                    "unused",
                    "--out",
                    "unused",
                ]
            )
            document = strategy_v9_evaluate.evaluate(cells, args)
            instrument = document["instruments"]["BTCUSD"]
            self.assertAlmostEqual(instrument["return"], 1.10 * 0.95 - 1.0)
            self.assertEqual(instrument["trades"], 75)
            self.assertAlmostEqual(instrument["profit_factor"], 15.0 / 15.0)
            self.assertEqual(document["pooled"]["positive_instruments"], 1)
            self.assertTrue(document["criteria"]["max_drawdown"])
            self.assertTrue(document["criteria"]["return"])
            self.assertFalse(document["criteria"]["profit_factor"])

    def test_require_no_halt_adds_the_criterion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_cell(root / "btc-2019", 0.10, 0.05, 40, [10.0], halted=True)
            cells = [
                {"instrument": "BTCUSD", "fold": 2019, "run": str(root / "btc-2019")}
            ]
            args = strategy_v9_evaluate.parse_arguments(
                [
                    "--matrix",
                    "unused",
                    "--out",
                    "unused",
                    "--require-no-halt",
                ]
            )
            document = strategy_v9_evaluate.evaluate(cells, args)
            self.assertFalse(document["criteria"]["no_halt"])
            self.assertFalse(document["passed"])

    def test_a_halted_fold_fails_the_drawdown_criterion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_cell(root / "btc-2019", 0.10, 0.1005, 40, [10.0])
            cells = [
                {"instrument": "BTCUSD", "fold": 2019, "run": str(root / "btc-2019")}
            ]
            args = strategy_v9_evaluate.parse_arguments(
                ["--matrix", "unused", "--out", "unused"]
            )
            document = strategy_v9_evaluate.evaluate(cells, args)
            self.assertFalse(document["criteria"]["max_drawdown"])
            self.assertFalse(document["passed"])


if __name__ == "__main__":
    unittest.main()

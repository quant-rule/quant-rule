#!/usr/bin/env python3
"""Descriptive metrics for one unified-ladder experiment cell.

This is an evaluation helper, not part of the product. It reads the portfolio
trades and equity CSVs and reports full-run and filtered out-of-sample metrics.
It uses only the Python standard library and is deterministic.

Usage:
    python3 docs/experiments/2026-10-30/analyze_unified_ladder.py <cell-dir>

The split is fixed at 2025-09-01 UTC for EXP-2026-10-30-02.
"""

import csv
import datetime as dt
import json
import statistics
import sys
from pathlib import Path

SPLIT = dt.date(2025, 9, 1)


def parse_trades(path):
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def parse_equity(path):
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def money(value):
    return float(value)


def trade_metrics(trades):
    if not trades:
        return {
            "trades": 0,
            "net_pnl": 0.0,
            "gross_pnl": 0.0,
            "commission": 0.0,
            "execution_fees": 0.0,
            "funding": 0.0,
            "wins": 0,
            "losses": 0,
            "win_rate": None,
            "profit_factor": None,
            "average_trade": None,
            "cost_drag": 0.0,
        }
    net = [money(row["net_pnl"]) for row in trades]
    gross = [money(row["gross_pnl"]) for row in trades]
    commission = [money(row["commission"]) for row in trades]
    fees = [money(row["execution_fees"]) for row in trades]
    funding = [money(row["funding"]) for row in trades]
    wins = [value for value in net if value > 0.0]
    losses = [value for value in net if value < 0.0]
    win_sum = sum(wins)
    loss_sum = abs(sum(losses))
    return {
        "trades": len(trades),
        "net_pnl": sum(net),
        "gross_pnl": sum(gross),
        "commission": sum(commission),
        "execution_fees": sum(fees),
        "funding": sum(funding),
        "wins": len(wins),
        "losses": len(losses),
        "win_rate": len(wins) / len(trades) if trades else None,
        "profit_factor": (win_sum / loss_sum) if loss_sum else None,
        "average_trade": statistics.fmean(net) if net else None,
        "cost_drag": sum(commission) + sum(fees) + sum(funding),
    }


def equity_metrics(points):
    if not points:
        return {
            "first_equity": None,
            "last_equity": None,
            "return": None,
            "max_drawdown": None,
            "max_drawdown_days": None,
        }
    values = [money(row["equity"]) for row in points]
    first = values[0]
    last = values[-1]
    peak = values[0]
    max_drawdown = 0.0
    peak_index = 0
    max_duration = 0
    for index, value in enumerate(values):
        if value > peak:
            peak = value
            peak_index = index
        drawdown = (peak - value) / peak if peak else 0.0
        if drawdown > max_drawdown:
            max_drawdown = drawdown
            max_duration = index - peak_index
    return {
        "first_equity": first,
        "last_equity": last,
        "return": (last / first - 1.0) if first else None,
        "max_drawdown": max_drawdown,
        "max_drawdown_days": max_duration,
    }


def main():
    if len(sys.argv) != 2:
        print("usage: analyze_unified_ladder.py <cell-dir>", file=sys.stderr)
        return 2
    cell = Path(sys.argv[1])
    trades = parse_trades(cell / "portfolio" / "trades.csv")
    equity = parse_equity(cell / "portfolio" / "equity.csv")

    full = trade_metrics(trades)
    oos_trades = [
        row
        for row in trades
        if dt.datetime.fromtimestamp(int(row["entry_time"]), dt.timezone.utc).date() >= SPLIT
    ]
    oos = trade_metrics(oos_trades)
    oos_points = [
        row for row in equity if dt.date.fromisoformat(row["date"]) >= SPLIT
    ]
    oos_equity = equity_metrics(oos_points)

    result = {
        "cell": cell.name,
        "data_version": "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31",
        "split": "2025-09-01",
        "full": {**full, **equity_metrics(equity)},
        "out_of_sample": {**oos, **oos_equity},
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

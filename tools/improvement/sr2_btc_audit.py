#!/usr/bin/env python3
"""Audit the SR2-A BTCUSD result for concentration, costs, and drawdown.

The audit is read-only over the recorded SR2-A BTCUSD cells and the exact
9 bps / 5% drawdown sensitivity bundles. It writes deterministic JSON, CSV, and
Markdown evidence. Statistics is evaluation only; nothing here enters a model,
a strategy, or a decision.

Usage:
    python3 tools/improvement/sr2_btc_audit.py \
        --base data/improvement/2026-11-08/STRATEGY-2026-11-08-02/artifacts/A \
        --audit data/improvement/2026-11-08/SR2-BTC-AUDIT \
        --out data/improvement/2026-11-08/SR2-BTC-AUDIT/analysis
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import statistics
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

EVENT_TYPE = 3
EVENT_SUBTYPE = 7
EVENT_SOURCE = 5
HORIZON = 20
COST_BPS = 6.0
BOOTSTRAP_DRAWS = 10_000
BOOTSTRAP_SEED = 0
CELLS = tuple(f"btc-{year}" for year in range(2018, 2023))


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_trades(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def day(timestamp: int) -> str:
    return datetime.fromtimestamp(timestamp, tz=timezone.utc).date().isoformat()


def metrics(root: Path, cell: str) -> dict[str, Any]:
    path = root / cell / "backtest" / "portfolio" / "metrics.json"
    return read_json(path)


def trades(root: Path, cell: str) -> list[dict[str, str]]:
    path = root / cell / "backtest" / "portfolio" / "trades.csv"
    return read_trades(path)


def aggregate(root: Path) -> dict[str, Any]:
    rows = [metrics(root, cell) for cell in CELLS]
    all_trades = [trade for cell in CELLS for trade in trades(root, cell)]
    net = [float(trade["net_pnl"]) for trade in all_trades]
    profit = sum(value for value in net if value > 0.0)
    loss = sum(value for value in net if value < 0.0)
    return {
        "cells": len(rows),
        "trades": sum(int(row["trades"]) for row in rows),
        "net_pnl": sum(float(row["net_pnl"]) for row in rows),
        "gross_pnl": sum(float(row["gross_pnl"]) for row in rows),
        "commission": sum(float(row["commission"]) for row in rows),
        "max_drawdown": max(float(row["max_drawdown"]) for row in rows),
        "halted_cells": sum(1 for row in rows if row["halted"]),
        "profit_factor": None if loss == 0.0 else profit / -loss,
        "per_year": [
            {
                "cell": cell,
                "trades": int(row["trades"]),
                "net_pnl": float(row["net_pnl"]),
                "max_drawdown": float(row["max_drawdown"]),
                "profit_factor": row["profit_factor"],
                "halted": bool(row["halted"]),
            }
            for cell, row in zip(CELLS, rows)
        ],
    }


def top_trade_table(root: Path) -> list[dict[str, Any]]:
    rows = []
    for cell in CELLS:
        for trade in trades(root, cell):
            rows.append(
                {
                    "cell": cell,
                    "trade_id": int(trade["trade_id"]),
                    "side": trade["side"],
                    "entry_time": day(int(trade["entry_time"])),
                    "exit_time": day(int(trade["exit_time"])),
                    "entry_price": float(trade["entry_price"]),
                    "exit_price": float(trade["exit_price"]),
                    "net_pnl": float(trade["net_pnl"]),
                    "return_fraction": float(trade["return_fraction"]),
                    "hold_bars": int(trade["hold_bars"]),
                    "exit_reason": trade["exit_reason"],
                }
            )
    rows.sort(key=lambda row: row["net_pnl"], reverse=True)
    return rows


def concentration(values: list[float]) -> dict[str, Any]:
    total = sum(values)
    ordered = sorted(values, reverse=True)
    result: dict[str, Any] = {"trades": len(values), "total": total}
    for count in (1, 3, 5, 10):
        result[f"without_top_{count}"] = sum(ordered[count:])
    result["top_5_share"] = (
        sum(ordered[:5]) / total if total != 0.0 else None
    )
    return result


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = fraction * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def bootstrap(values: list[float]) -> dict[str, Any]:
    rng = random.Random(BOOTSTRAP_SEED)
    count = len(values)
    means: list[float] = []
    totals: list[float] = []
    for _ in range(BOOTSTRAP_DRAWS):
        sample = [values[rng.randrange(count)] for _ in range(count)]
        means.append(sum(sample) / count)
        totals.append(sum(sample))
    return {
        "draws": BOOTSTRAP_DRAWS,
        "seed": BOOTSTRAP_SEED,
        "mean": sum(values) / count,
        "mean_ci_95": [percentile(means, 0.025), percentile(means, 0.975)],
        "total_ci_95": [percentile(totals, 0.025), percentile(totals, 0.975)],
    }


def t_stat(values: list[float]) -> float:
    if len(values) < 2:
        return 0.0
    mean = sum(values) / len(values)
    deviation = statistics.pstdev(values)
    if deviation == 0.0:
        return 0.0
    return mean / (deviation / math.sqrt(len(values)))


def read_readings(path: Path) -> list[dict[str, Any]]:
    rows = []
    for row in csv.DictReader(path.open(encoding="utf-8", newline="")):
        try:
            rows.append(
                {
                    "bar": int(row["bar"]),
                    "timestamp": int(row["timestamp"]),
                    "open": float(row["open"]),
                    "high": float(row["high"]),
                    "low": float(row["low"]),
                    "close": float(row["close"]),
                    "atr": float(row["atr"]) if row["atr"] else None,
                    "ema_stack": int(row["ema_stack"]) if row["ema_stack"] else None,
                    "atr_ratio": float(row["atr_ratio"]) if row["atr_ratio"] else None,
                    "rvol_ratio": float(row["rvol_ratio"]) if row["rvol_ratio"] else None,
                }
            )
        except (KeyError, TypeError, ValueError):
            continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_events(path: Path, timeframe_secs: int) -> list[dict[str, Any]]:
    events = []
    for row in csv.DictReader(path.open(encoding="utf-8", newline="")):
        try:
            if int(row["timeframe_secs"]) != timeframe_secs:
                continue
            if int(row["event_type_code"]) != EVENT_TYPE:
                continue
            if int(row["event_subtype_code"]) != EVENT_SUBTYPE:
                continue
            if int(row["source_code"]) != EVENT_SOURCE:
                continue
            events.append(
                {
                    "available": int(row["available_bar_index"]),
                    "direction": int(row["direction_code"]),
                }
            )
        except (KeyError, TypeError, ValueError):
            continue
    events.sort(key=lambda event: event["available"])
    return events


def path_value(
    rows: list[dict[str, Any]],
    event: dict[str, Any],
    target_atr: float,
    cost_bps: float,
) -> dict[str, Any] | None:
    index = event["available"] + 1
    if index >= len(rows):
        return None
    entry = rows[index]["open"]
    atr = rows[event["available"]]["atr"]
    if not atr or atr <= 0.0 or entry <= 0.0:
        return None
    side = -event["direction"]
    if side == 0:
        return None
    stop = entry - side * atr
    target = entry + side * target_atr * atr
    last = min(index + HORIZON, len(rows) - 1)
    reason = "time"
    value = 0.0
    hold = last - index
    for position in range(index, last + 1):
        bar = rows[position]
        if side > 0:
            if bar["low"] <= stop:
                value, reason, hold = -1.0, "stop", position - index
                break
            if bar["high"] >= target:
                value, reason, hold = target_atr, "target", position - index
                break
        else:
            if bar["high"] >= stop:
                value, reason, hold = -1.0, "stop", position - index
                break
            if bar["low"] <= target:
                value, reason, hold = target_atr, "target", position - index
                break
    else:
        close = rows[last]["close"]
        value = side * (close - entry) / atr
    cost_atr = cost_bps / 10_000.0 * entry / atr
    return {
        "value": value - cost_atr,
        "reason": reason,
        "hold": hold,
        "side": "long" if side > 0 else "short",
        "year": int(datetime.fromtimestamp(rows[event["available"]]["timestamp"], tz=timezone.utc).year),
        "hour": datetime.fromtimestamp(rows[event["available"]]["timestamp"], tz=timezone.utc).hour,
        "ema_stack": rows[event["available"]]["ema_stack"],
        "atr_ratio": rows[event["available"]]["atr_ratio"],
        "rvol_ratio": rows[event["available"]]["rvol_ratio"],
    }


def summarize(values: list[dict[str, Any]]) -> dict[str, Any]:
    if not values:
        return {"n": 0}
    returns = [value["value"] for value in values]
    count = len(returns)
    return {
        "n": count,
        "mean_r": sum(returns) / count,
        "t_stat": t_stat(returns),
        "target_share": sum(1 for value in values if value["reason"] == "target") / count,
        "stop_share": sum(1 for value in values if value["reason"] == "stop") / count,
        "time_share": sum(1 for value in values if value["reason"] == "time") / count,
    }


def path_analysis(base: Path) -> dict[str, Any]:
    values: list[dict[str, Any]] = []
    for cell in CELLS:
        model = base / cell / "backtest" / "model"
        rows = read_readings(model / "readings.csv")
        events = read_events(model / "events.csv", 3_600)
        for event in events:
            value = path_value(rows, event, 3.0, COST_BPS)
            if value is not None:
                values.append(value)
    buckets: dict[str, dict[str, Any]] = {"all": summarize(values)}
    for key in ("year", "side", "ema_stack"):
        grouped: dict[str, list[dict[str, Any]]] = {}
        for value in values:
            grouped.setdefault(str(value[key]), []).append(value)
        buckets[key] = {name: summarize(group) for name, group in sorted(grouped.items())}
    for key in ("atr_ratio", "rvol_ratio"):
        finite = [value[key] for value in values if value[key] is not None]
        if not finite:
            continue
        median = statistics.median(finite)
        grouped = {}
        for value in values:
            if value[key] is None:
                continue
            name = "high" if value[key] >= median else "low"
            grouped.setdefault(name, []).append(value)
        buckets[key] = {
            "median": median,
            **{name: summarize(group) for name, group in sorted(grouped.items())},
        }
    grouped = {}
    for value in values:
        if value["hour"] < 6:
            name = "00-06"
        elif value["hour"] < 12:
            name = "06-12"
        elif value["hour"] < 18:
            name = "12-18"
        else:
            name = "18-24"
        grouped.setdefault(name, []).append(value)
    buckets["session_utc"] = {
        name: summarize(group) for name, group in sorted(grouped.items())
    }
    return buckets


def goal_gap(net_pnl: float, max_drawdown: float) -> dict[str, Any]:
    total_return = net_pnl / 50_000.0
    monthly = (1.0 + total_return) ** (1.0 / 60.0) - 1.0 if total_return > -1.0 else None
    required = 1.10**60 - 1.0
    return {
        "total_return_5y": total_return,
        "monthly_equivalent": monthly,
        "max_drawdown": max_drawdown,
        "required_10pct_monthly_total": required,
        "gap_multiple": (1.0 + required) / (1.0 + total_return)
        if total_return > -1.0
        else None,
    }


def write_csv(path: Path, rows: list[dict[str, Any]], fields: list[str]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="SR2-A artifacts root")
    parser.add_argument("--audit", required=True, help="audit run root")
    parser.add_argument("--out", required=True, help="analysis output directory")
    args = parser.parse_args(argv[1:])
    base = Path(args.base).resolve()
    audit = Path(args.audit).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)

    base_result = aggregate(base)
    sensitivity = aggregate(audit / "artifacts-9bps")
    drawdown = aggregate(audit / "artifacts-5pct")

    all_trades = top_trade_table(base)
    values = [trade["net_pnl"] for trade in all_trades]
    result = {
        "subject": "SR2-A BTCUSD 1h 1:3 fade, 2018-2022",
        "base": base_result,
        "sensitivity_9bps": sensitivity,
        "drawdown_5pct": drawdown,
        "top_trades": all_trades[:10],
        "concentration": concentration(values),
        "bootstrap": bootstrap(values),
        "t_stat": t_stat(values),
        "path": path_analysis(base),
        "goal_gap": goal_gap(base_result["net_pnl"], base_result["max_drawdown"]),
        "limitations": [
            "The bucket analysis is exploratory and multiple-comparison-prone.",
            "The 5% drawdown halt is bar-close and the observed maximum can exceed 5%.",
            "One instrument and five years; 2024 is not opened.",
            "The strategy uses a context event as its sole entry; this is the SR1 exception.",
        ],
    }
    (out / "analysis.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    write_csv(
        out / "top_trades.csv",
        all_trades[:10],
        [
            "cell",
            "trade_id",
            "side",
            "entry_time",
            "exit_time",
            "entry_price",
            "exit_price",
            "net_pnl",
            "return_fraction",
            "hold_bars",
            "exit_reason",
        ],
    )
    bucket_rows = []
    for group, entries in result["path"].items():
        if group == "all":
            bucket_rows.append({"group": group, "bucket": "all", **entries})
            continue
        for bucket, summary in entries.items():
            if isinstance(summary, dict) and "n" in summary:
                bucket_rows.append({"group": group, "bucket": bucket, **summary})
    write_csv(
        out / "path_buckets.csv",
        bucket_rows,
        ["group", "bucket", "n", "mean_r", "t_stat", "target_share", "stop_share", "time_share"],
    )
    print(f"wrote {out / 'analysis.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

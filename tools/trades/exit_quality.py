#!/usr/bin/env python3
"""Describe exit quality for every trade in a recorded backtest.

The table is descriptive and evaluation-only. It never changes a strategy and
never feeds a model, event, or decision. All post-exit values are held in this
report; they are not written into the trade context's decision summary.

Usage:
    python3 tools/trades/exit_quality.py \
        --bundle data/.../backtest \
        --out data/.../exit_quality.csv
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

from bundle import (
    BundleError,
    load_bundle,
    timestamp_utc,
    write_csv,
    write_json,
)

POST_WINDOWS = (5, 10, 20, 50)


def nullable(value: float | None) -> float | str:
    """Use the repository's null spelling for an undefined number."""
    return "null" if value is None else value


def move_fraction(entry: float, high: float, low: float, side: str) -> tuple[float, float]:
    """Return (favorable, adverse) fractional moves for a side."""
    if side == "long":
        return high / entry - 1.0, low / entry - 1.0
    return 1.0 - low / entry, 1.0 - high / entry


def post_moves(bundle, start_bar: int, end_bar: int, side: str, reference: float):
    """Return post-exit favorable/adverse moves and ATR values."""
    rows = [
        row
        for row in bundle.readings
        if start_bar <= int(row["bar"]) <= end_bar
    ]
    if not rows:
        return None, None
    high = max(float(row["high"]) for row in rows)
    low = min(float(row["low"]) for row in rows)
    favorable, adverse = move_fraction(reference, high, low, side)
    return favorable, adverse


def trigger_prices(bundle, trade: dict[str, str]) -> tuple[float | None, float | None]:
    """Return the recorded stop and target trigger prices."""
    stop = target = None
    for order in bundle.trade_orders(trade):
        reason = order.get("reason")
        try:
            price = float(order["trigger_price"])
        except (KeyError, ValueError):
            continue
        if reason == "stop_loss" and stop is None:
            stop = price
        elif reason == "take_profit" and target is None:
            target = price
    return stop, target


def would_hit(rows: list[dict[str, str]], target: float | None, stop: float | None, side: str):
    """Return whether a later bar would have touched the target or stop."""
    target_hit = stop_hit = None
    if target is not None:
        if side == "long":
            target_hit = any(float(row["high"]) >= target for row in rows)
        else:
            target_hit = any(float(row["low"]) <= target for row in rows)
    if stop is not None:
        if side == "long":
            stop_hit = any(float(row["low"]) <= stop for row in rows)
        else:
            stop_hit = any(float(row["high"]) >= stop for row in rows)
    return target_hit, stop_hit


def quality_row(bundle, trade: dict[str, str]) -> dict[str, object]:
    """Build one exit-quality row."""
    entry_bar = int(trade["entry_bar"])
    exit_bar = int(trade["exit_bar"])
    side = trade["side"]
    entry_price = float(trade["entry_price"])
    exit_price = float(trade["exit_price"])
    exit_decision_bar = exit_bar - 1
    exit_reading = next(
        row for row in bundle.readings if int(row["bar"]) == exit_decision_bar
    )
    atr_exit = float(exit_reading["atr"])
    trade_rows = [
        row
        for row in bundle.readings
        if entry_bar <= int(row["bar"]) <= exit_bar
    ]
    if trade_rows:
        trade_high = max(float(row["high"]) for row in trade_rows)
        trade_low = min(float(row["low"]) for row in trade_rows)
        mfe_trade, mae_trade = move_fraction(entry_price, trade_high, trade_low, side)
    else:
        mfe_trade = mae_trade = None
    captured = float(trade["return_fraction"])
    efficiency = (
        captured / mfe_trade
        if mfe_trade is not None and mfe_trade > 0.0
        else None
    )
    row: dict[str, object] = {
        "trade_id": int(trade["trade_id"]),
        "side": side,
        "entry_time": timestamp_utc(int(trade["entry_time"])),
        "exit_time": timestamp_utc(int(trade["exit_time"])),
        "entry_bar": entry_bar,
        "exit_bar": exit_bar,
        "exit_reason": trade["exit_reason"],
        "captured_return": captured,
        "atr_exit": atr_exit,
        "mfe_trade": nullable(mfe_trade),
        "mae_trade": nullable(mae_trade),
        "exit_efficiency": nullable(efficiency),
        "exit_decision": (bundle.decision_at(exit_decision_bar) or {}).get("decision"),
    }
    for field in (
        "ema_stack",
        "structure_state",
        "vol_regime",
        "rsi",
        "rsi_zone",
        "macd_state",
        "macd_histogram",
        "adx",
        "adx_regime",
        "rvol_ratio",
        "rvol_regime",
        "atr_ratio",
        "vwap_distance_atr",
        "range_position",
        "drawdown_atr",
        "path_efficiency",
        "level_touches",
    ):
        value = exit_reading.get(field)
        row[f"exit_{field}"] = "null" if value in (None, "") else float(value)
    stop, target = trigger_prices(bundle, trade)
    row["stop"] = nullable(stop)
    row["target"] = nullable(target)
    for window in POST_WINDOWS:
        post = [
            item
            for item in bundle.readings
            if exit_bar + 1 <= int(item["bar"]) <= exit_bar + window
        ]
        if not post:
            row[f"post_{window}_mfe"] = "null"
            row[f"post_{window}_mae"] = "null"
            row[f"post_{window}_mfe_atr"] = "null"
            row[f"post_{window}_mae_atr"] = "null"
            row[f"post_{window}_target_hit"] = "null"
            row[f"post_{window}_stop_hit"] = "null"
            continue
        favorable, adverse = post_moves(bundle, exit_bar + 1, exit_bar + window, side, exit_price)
        row[f"post_{window}_mfe"] = nullable(favorable)
        row[f"post_{window}_mae"] = nullable(adverse)
        row[f"post_{window}_mfe_atr"] = (
            "null" if favorable is None or atr_exit == 0.0 else favorable * exit_price / atr_exit
        )
        row[f"post_{window}_mae_atr"] = (
            "null" if adverse is None or atr_exit == 0.0 else adverse * exit_price / atr_exit
        )
        target_hit, stop_hit = would_hit(post, target, stop, side)
        row[f"post_{window}_target_hit"] = target_hit
        row[f"post_{window}_stop_hit"] = stop_hit
    return row


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--timeframe-secs", type=int, default=900)
    args = parser.parse_args(argv[1:])
    try:
        bundle = load_bundle(args.bundle, args.timeframe_secs)
        rows = [quality_row(bundle, trade) for trade in bundle.trades]
        # Keep a stable column order: the first row defines it, and every row
        # has the same fields.
        write_csv(args.out, rows)
        write_json(
            args.out.with_suffix(".json"),
            {
                "schema": "quantrule.exit-quality/v1",
                "bundle": str(args.bundle),
                "data_version": bundle.manifest.get("data_version"),
                "config_id": bundle.manifest.get("config_id"),
                "trades": rows,
                "post_windows": list(POST_WINDOWS),
            },
        )
    except (BundleError, ValueError, KeyError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {len(rows)} exit-quality rows to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

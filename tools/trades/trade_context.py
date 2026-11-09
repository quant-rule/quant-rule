#!/usr/bin/env python3
"""Export a 250-bar context bundle for every trade in a recorded backtest.

The exporter is read-only over the backtest bundle. It writes the last 250 bars
ending at each entry/exit decision bar, a separate post-fill window, the events
in the context window, the strategy decision and evidence, the linked orders and
fills, and a full all-column summary at the decision bar.

Usage:
    python3 tools/trades/trade_context.py \
        --bundle data/improvement/2026-11-09/BINANCE-BTCUSDT/artifacts/2018/9bps/backtest \
        --out data/improvement/2026-11-09/BINANCE-BTCUSDT/context/2018
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

from bundle import (
    Bundle,
    BundleError,
    event_catalog,
    event_label,
    load_bundle,
    number_or_none,
    reading_summary,
    timestamp_utc,
    write_csv,
    write_json,
)

DEFAULT_WINDOW = 250
DEFAULT_POST = 250


def context_rows(bundle: Bundle, end_bar: int, window: int) -> list[dict[str, str]]:
    """Return the last ``window`` rows ending at or before ``end_bar``."""
    return bundle.bars_ending_at(end_bar, window)


def post_rows(bundle: Bundle, start_bar: int, post: int, entry: bool) -> list[dict[str, str]]:
    """Return the outcome window after a fill."""
    start = start_bar if entry else start_bar + 1
    return bundle.bars_starting_at(start, post)


def event_rows(bundle: Bundle, start_bar: int, end_bar: int) -> list[dict[str, str]]:
    """Return events available in the context window."""
    return bundle.events_between(start_bar, end_bar)


def order_rows(bundle: Bundle, trade: dict[str, str], leg: str) -> list[dict[str, str]]:
    """Return the orders linked to one leg."""
    orders = bundle.trade_orders(trade)
    if leg == "entry":
        return [order for order in orders if order.get("reason") == "entry"]
    return [order for order in orders if order.get("reason") != "entry"]


def fill_rows(bundle: Bundle, trade: dict[str, str], leg: str) -> list[dict[str, str]]:
    """Return the fills linked to one leg."""
    fills = bundle.trade_fills(trade)
    bar = int(trade["entry_bar"] if leg == "entry" else trade["exit_bar"])
    return [fill for fill in fills if int(fill["bar"]) == bar]


def export_leg(
    bundle: Bundle,
    trade: dict[str, str],
    leg: str,
    out: Path,
    window: int,
    post: int,
    catalog: dict[int, dict[str, Any]],
) -> dict[str, Any]:
    """Export one entry or exit context bundle."""
    if leg == "entry":
        fill_bar = int(trade["entry_bar"])
        fill_time = int(trade["entry_time"])
        fill_price = float(trade["entry_price"])
    else:
        fill_bar = int(trade["exit_bar"])
        fill_time = int(trade["exit_time"])
        fill_price = float(trade["exit_price"])
    decision_bar = fill_bar - 1
    rows = context_rows(bundle, decision_bar, window)
    if not rows:
        raise BundleError(f"no bars at or before decision bar {decision_bar}")
    start_bar = int(rows[0]["bar"])
    end_bar = int(rows[-1]["bar"])
    if end_bar != decision_bar:
        raise BundleError(
            f"context for trade {trade['trade_id']} {leg} ends at {end_bar}, "
            f"not the decision bar {decision_bar}"
        )
    post_window = post_rows(bundle, fill_bar, post, leg == "entry")
    events = event_rows(bundle, start_bar, end_bar)
    decision = bundle.decision_at(decision_bar)
    orders = order_rows(bundle, trade, leg)
    fills = fill_rows(bundle, trade, leg)
    leg_root = out / leg
    write_csv(leg_root / "context.csv", rows)
    write_csv(leg_root / "post.csv", post_window)
    write_csv(leg_root / "events.csv", events)
    write_json(leg_root / "decision.json", decision)
    write_json(leg_root / "orders.json", orders)
    write_json(leg_root / "fills.json", fills)
    summary = {
        "schema": "quantrule.trade-context-summary/v1",
        "trade_id": int(trade["trade_id"]),
        "side": trade["side"],
        "leg": leg,
        "decision_bar": decision_bar,
        "decision_time": timestamp_utc(int(rows[-1]["timestamp"])),
        "fill_bar": fill_bar,
        "fill_time": timestamp_utc(fill_time),
        "fill_price": fill_price,
        "readings": reading_summary(rows[-1]),
        "events_at_decision": [
            {
                "code": int(event["event_code"]),
                "id": event_label(int(event["event_code"]), catalog),
                "type": int(event["event_type_code"]),
                "subtype": int(event["event_subtype_code"]),
                "source": int(event["source_code"]),
                "direction": int(event["direction_code"]),
                "occurred_bar": int(event["occurred_bar_index"]),
                "available_bar": int(event["available_bar_index"]),
                "values": {
                    "primary": number_or_none(event["primary_value"]),
                    "reference": number_or_none(event["reference_value"]),
                    "completeness": number_or_none(event["data_completeness"]),
                },
            }
            for event in bundle.events_at(decision_bar)
        ],
        "decision": decision,
        "orders": orders,
        "fills": fills,
        "context_bars": len(rows),
        "context_start": timestamp_utc(int(rows[0]["timestamp"])),
        "context_end": timestamp_utc(int(rows[-1]["timestamp"])),
        "post_bars": len(post_window),
        "post_start": timestamp_utc(int(post_window[0]["timestamp"])) if post_window else None,
        "post_end": timestamp_utc(int(post_window[-1]["timestamp"])) if post_window else None,
    }
    write_json(leg_root / "summary.json", summary)
    return {
        "leg": leg,
        "decision_bar": decision_bar,
        "fill_bar": fill_bar,
        "decision_time": summary["decision_time"],
        "fill_time": summary["fill_time"],
        "context_bars": len(rows),
        "post_bars": len(post_window),
        "events": len(events),
    }


def export_trade(
    bundle: Bundle,
    trade: dict[str, str],
    out: Path,
    window: int,
    post: int,
    catalog: dict[int, dict[str, Any]],
) -> dict[str, Any]:
    """Export both legs of one trade."""
    # The run label comes from the bundle path where possible.
    year = next((part for part in reversed(bundle.root.parts) if part.isdigit() and len(part) == 4), None)
    trade_root = out / f"{year + '-' if year is not None else ''}trade-{trade['trade_id']}"
    trade_root.mkdir(parents=True, exist_ok=True)
    trade_record = {
        "schema": "quantrule.trade-context/v1",
        "trade": trade,
        "data_version": bundle.manifest.get("data_version"),
        "config_id": bundle.manifest.get("config_id"),
        "timeframe_secs": bundle.timeframe_secs,
        "run": bundle.manifest.get("run"),
    }
    write_json(trade_root / "trade.json", trade_record)
    entry = export_leg(bundle, trade, "entry", trade_root, window, post, catalog)
    exit_ = export_leg(bundle, trade, "exit", trade_root, window, post, catalog)
    return {
        "trade_id": int(trade["trade_id"]),
        "side": trade["side"],
        "entry_time": timestamp_utc(int(trade["entry_time"])),
        "exit_time": timestamp_utc(int(trade["exit_time"])),
        "net_pnl": float(trade["net_pnl"]),
        "exit_reason": trade["exit_reason"],
        "path": str(trade_root),
        "entry": entry,
        "exit": exit_,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--window", type=int, default=DEFAULT_WINDOW)
    parser.add_argument("--post", type=int, default=DEFAULT_POST)
    parser.add_argument("--timeframe-secs", type=int, default=900)
    parser.add_argument("--event-registry", type=Path, default=Path("docs/registry/indicators.json"))
    parser.add_argument("--trade-id", type=int, action="append")
    args = parser.parse_args(argv[1:])
    if args.window < 1 or args.post < 0:
        print("error: --window must be positive and --post non-negative", file=sys.stderr)
        return 1
    try:
        bundle = load_bundle(args.bundle, args.timeframe_secs)
        catalog = event_catalog(args.event_registry)
        trades = bundle.trades
        if args.trade_id:
            wanted = set(args.trade_id)
            trades = [trade for trade in trades if int(trade["trade_id"]) in wanted]
        if not trades:
            raise BundleError("no trades selected")
        records = [
            export_trade(bundle, trade, args.out, args.window, args.post, catalog)
            for trade in trades
        ]
        index = {
            "schema": "quantrule.trade-context-index/v1",
            "bundle": str(args.bundle),
            "data_version": bundle.manifest.get("data_version"),
            "config_id": bundle.manifest.get("config_id"),
            "timeframe_secs": bundle.timeframe_secs,
            "window": args.window,
            "post": args.post,
            "trades": records,
        }
        write_json(args.out / "index.json", index)
    except (BundleError, ValueError, KeyError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {len(records)} trade contexts to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

#!/usr/bin/env python3
"""Barrier-path diagnostic for the 5m fresh-onset compression event.

The harness joins each validated onset event at its availability bar, resolves
the entry population exactly as the v10 bracket does, and scans the next 20 bars
for a target-before-stop path. It measures both the momentum side (the bracket's
direction) and the fade side for 1, 2, 3, and 4 ATR targets against a 1 ATR stop.

Statistics is evaluation only. Nothing here enters the model or a strategy.

The protocol is `docs/model/2026-11-04/barrier-path-5m-onset.md`.

Usage:
    python3 tools/improvement/barrier_path.py \
        --matrix config/compression-range-onset-5m/dev/matrix.json \
        --out data/improvement/2026-11-04/MODEL-2026-11-04-12/artifacts
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

from range_validation import (
    read_range_events,
    read_rows_for_timeframe,
    select_events,
    window_hits,
)

EVENT_SUBTYPE = 9
TIMEFRAME = 300
COILING_BARS = 5
HORIZON = 20
STOP_ATR = 1.0
TARGETS = (1.0, 2.0, 3.0, 4.0)
COST_BPS = 12.0
T8 = 2.306
MIN_INSTRUMENT_EVENTS = 30


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, help="matrix JSON")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--horizon", type=int, default=HORIZON)
    parser.add_argument("--cost-bps", type=float, default=COST_BPS)
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def entry_for_event(
    rows: list[dict[str, Any]], event: dict[str, Any]
) -> tuple[int, int, float, float] | None:
    """Returns (entry_index, direction, entry_price, atr) or None."""
    bar = event["available"]
    if bar + 1 >= len(rows):
        return None
    atr = rows[bar]["atr"]
    if atr is None or not math.isfinite(atr) or atr <= 0.0:
        return None
    upper = event.get("primary")
    lower = event.get("reference")
    if upper is None or lower is None or upper <= lower:
        return None
    nxt = rows[bar + 1]
    if nxt["open"] >= upper:
        return bar + 1, 1, nxt["open"], atr
    if nxt["open"] <= lower:
        return bar + 1, -1, nxt["open"], atr
    long_touch = nxt["high"] >= upper
    short_touch = nxt["low"] <= lower
    if long_touch and short_touch:
        return None
    if long_touch:
        return bar + 1, 1, upper, atr
    if short_touch:
        return bar + 1, -1, lower, atr
    return None


def scan_path(
    rows: list[dict[str, Any]],
    entry_index: int,
    direction: int,
    entry: float,
    atr: float,
    target_atr: float,
    horizon: int,
    target_on_entry_bar: bool = True,
) -> tuple[str, float, int]:
    """Scans for a target before a stop; returns (outcome, r, exit_index)."""
    stop = entry - direction * STOP_ATR * atr
    target = entry + direction * target_atr * atr
    if entry_index >= len(rows):
        return "timeout", 0.0, entry_index
    for offset in range(horizon):
        index = entry_index + offset
        if index >= len(rows):
            break
        row = rows[index]
        if offset == 0:
            # The position did not exist at the entry bar's open; evaluate the
            # bar range from the fill and keep the stop-first tie rule.
            if direction > 0:
                if row["low"] <= stop:
                    return "stop", -1.0, index
                if target_on_entry_bar and row["high"] >= target:
                    return "target", target_atr, index
            else:
                if row["high"] >= stop:
                    return "stop", -1.0, index
                if target_on_entry_bar and row["low"] <= target:
                    return "target", target_atr, index
        else:
            if direction > 0:
                if row["open"] <= stop:
                    return "stop", -1.0, index
                if row["open"] >= target:
                    return "target", target_atr, index
                if row["low"] <= stop:
                    return "stop", -1.0, index
                if row["high"] >= target:
                    return "target", target_atr, index
            else:
                if row["open"] >= stop:
                    return "stop", -1.0, index
                if row["open"] <= target:
                    return "target", target_atr, index
                if row["high"] >= stop:
                    return "stop", -1.0, index
                if row["low"] <= target:
                    return "target", target_atr, index
    exit_index = min(entry_index + horizon - 1, len(rows) - 1)
    exit_price = rows[exit_index]["close"]
    r = direction * (exit_price - entry) / atr
    return "timeout", r, exit_index


def summarize(
    values: list[dict[str, Any]], cost_bps: float
) -> dict[str, Any]:
    if not values:
        return {
            "n": 0,
            "targets": 0,
            "stops": 0,
            "timeouts": 0,
            "target_rate": None,
            "stop_rate": None,
            "timeout_rate": None,
            "mean_gross_r": None,
            "mean_cost_r": None,
            "mean_net_r": None,
            "net_lcb": None,
            "positive_instruments": 0,
            "instruments": {},
        }
    n = len(values)
    targets = sum(1 for value in values if value["outcome"] == "target")
    stops = sum(1 for value in values if value["outcome"] == "stop")
    timeouts = n - targets - stops
    gross = [value["r"] for value in values]
    cost = [value["cost_r"] for value in values]
    net = [value["r"] - value["cost_r"] for value in values]
    bundle_net: dict[tuple[str, int], list[float]] = defaultdict(list)
    by_instrument: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for value in values:
        bundle_net[(value["instrument"], value["fold"])].append(value["net_r"])
        by_instrument[value["instrument"]].append(value)
    bundles = [sum(items) / len(items) for items in bundle_net.values()]
    mean_net = sum(net) / n
    if len(bundles) > 1:
        stdev = statistics.stdev(bundles)
        lcb = mean_net - T8 * stdev / math.sqrt(len(bundles))
    else:
        lcb = None
    instruments = {}
    positive = 0
    for instrument, items in sorted(by_instrument.items()):
        mean = sum(item["net_r"] for item in items) / len(items)
        instruments[instrument] = {"n": len(items), "mean_net_r": mean}
        if mean > 0.0 and len(items) >= MIN_INSTRUMENT_EVENTS:
            positive += 1
    return {
        "n": n,
        "targets": targets,
        "stops": stops,
        "timeouts": timeouts,
        "target_rate": targets / n,
        "stop_rate": stops / n,
        "timeout_rate": timeouts / n,
        "mean_gross_r": sum(gross) / n,
        "mean_cost_r": sum(cost) / n,
        "mean_net_r": mean_net,
        "net_lcb": lcb,
        "positive_instruments": positive,
        "instruments": instruments,
    }


def evaluate(
    matrix: dict[str, Any], horizon: int, cost_bps: float
) -> dict[str, Any]:
    records: dict[tuple[str, float], list[dict[str, Any]]] = defaultdict(list)
    entries = 0
    skipped = 0
    for cell in matrix["cells"]:
        instrument = str(cell["instrument"])
        fold = int(cell["fold"])
        run = Path(cell["run"]).expanduser()
        rows = read_rows_for_timeframe(run / "readings.csv", TIMEFRAME)
        events = read_range_events(run / "events.csv", TIMEFRAME, EVENT_SUBTYPE)
        hits, highs, lows = window_hits(rows, COILING_BARS)
        selected, _counts = select_events(events, hits, highs, lows)
        for event in selected:
            resolved = entry_for_event(rows, event)
            if resolved is None:
                skipped += 1
                continue
            entry_index, momentum, entry, atr = resolved
            entries += 1
            cost_r = cost_bps / 10_000.0 * entry / atr
            for direction, side_name in ((momentum, "momentum"), (-momentum, "fade")):
                for target_atr in TARGETS:
                    outcome, r, _exit = scan_path(
                        rows,
                        entry_index,
                        direction,
                        entry,
                        atr,
                        target_atr,
                        horizon,
                        side_name == "momentum",
                    )
                    records[(side_name, target_atr)].append(
                        {
                            "instrument": instrument,
                            "fold": fold,
                            "r": r,
                            "outcome": outcome,
                            "cost_r": cost_r,
                            "net_r": r - cost_r,
                        }
                    )
    results = []
    for side_name in ("momentum", "fade"):
        for target_atr in TARGETS:
            summary = summarize(records[(side_name, target_atr)], cost_bps)
            summary["side"] = side_name
            summary["target_atr"] = target_atr
            summary["promising"] = (
                summary["net_lcb"] is not None
                and summary["net_lcb"] > 0.0
                and summary["positive_instruments"] >= 2
            )
            results.append(summary)
    promising = sorted(
        [result for result in results if result["promising"]],
        key=lambda result: (
            -(result["net_lcb"] if result["net_lcb"] is not None else -math.inf),
            0 if result["side"] == "momentum" else 1,
            result["target_atr"],
        ),
    )
    if promising:
        best = promising[0]
        decision = "promising"
        recommendation = (
            f"{best['side']} side at {best['target_atr']:g} ATR target / 1 ATR stop"
        )
    else:
        decision = "no directional edge"
        recommendation = "stop the bracket program"
    return {
        "schema": "quantrule.barrier-path/v1",
        "partition": matrix.get("partition"),
        "timeframe": "5m",
        "event_subtype": EVENT_SUBTYPE,
        "horizon": horizon,
        "stop_atr": STOP_ATR,
        "targets": list(TARGETS),
        "cost_bps": cost_bps,
        "entries": entries,
        "skipped": skipped,
        "results": results,
        "decision": decision,
        "recommendation": recommendation,
    }


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:+.4f}"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# 5m Onset Barrier Path",
        "",
        f"- Partition: `{document['partition']}`",
        f"- Entries: {document['entries']}; skipped: {document['skipped']}",
        f"- Horizon: {document['horizon']} bars; stop: {document['stop_atr']} ATR",
        f"- Cost: {document['cost_bps']} bps round trip",
        f"- Decision: **{document['decision']}**",
        "",
        "| Side | Target | n | Target rate | Stop rate | Timeout | Gross R | Cost R | Net R | Net LCB | Positive | Promising |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for result in document["results"]:
        lines.append(
            "| {side} | {target:g} | {n} | {tr} | {sr} | {to} | {gross} | {cost} | {net} | {lcb} | {pos}/3 | {prom} |".format(
                side=result["side"],
                target=result["target_atr"],
                n=result["n"],
                tr=fmt(result["target_rate"]),
                sr=fmt(result["stop_rate"]),
                to=fmt(result["timeout_rate"]),
                gross=fmt(result["mean_gross_r"]),
                cost=fmt(result["mean_cost_r"]),
                net=fmt(result["mean_net_r"]),
                lcb=fmt(result["net_lcb"]),
                pos=result["positive_instruments"],
                prom="yes" if result["promising"] else "no",
            )
        )
    lines += [
        "",
        f"Recommendation: {document['recommendation']}.",
        "",
    ]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    matrix = read_json(Path(args.matrix).expanduser())
    if not isinstance(matrix, dict) or not isinstance(matrix.get("cells"), list):
        print("error: matrix must be an object with cells", file=sys.stderr)
        return 1
    if args.horizon <= 0:
        print("error: --horizon must be positive", file=sys.stderr)
        return 1
    document = evaluate(matrix, args.horizon, args.cost_bps)
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "barrier_path.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "barrier_path.md").write_text(markdown(document), encoding="utf-8")
    print(f"ok barrier path -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
